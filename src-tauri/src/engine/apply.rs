//! Recoverable apply protocol (NOT multi-file atomicity).
//!
//! Transaction protocol:
//! 1. Persist `journal.inprogress.json` with pre-apply snapshot backup BEFORE any mutation.
//! 2. For each row: backup → mutate → append journal entry → flush journal.
//! 3. On success: write committed snapshot, promote journal to `{id}.json`, set `latest.txt`.
//! 4. On crash: `recover_incomplete()` rolls back using backups + restores snapshot.
//!
//! Individual file replaces use temp+fsync+rename (Windows-safe). The whole batch is
//! recoverable, not all-or-nothing atomic across files.

use crate::engine::fsutil::{fsync_path, remove_file_resilient, replace_file, write_atomic};
use crate::engine::hash::hash_file;
use crate::engine::paths::{codesetu_data_dir, normalize_rel_path, safe_join};
use crate::engine::snapshot::{load_snapshot_from, save_snapshot, save_snapshot_to, Snapshot};
use crate::engine::types::{ChangeClass, Decision, FileRow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub rel_path: String,
    pub action: String,
    pub backup_path: Option<String>,
    pub previous_hash: Option<String>,
    pub new_hash: Option<String>,
    /// For moves: the old relative path that was removed.
    #[serde(default)]
    pub moved_from: Option<String>,
    #[serde(default)]
    pub moved_from_backup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum JournalStatus {
    InProgress,
    #[default]
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    pub session_id: String,
    pub this_pc: String,
    pub brought_over: String,
    pub created_at: String,
    #[serde(default)]
    pub status: JournalStatus,
    /// Path to JSON snapshot taken before first mutation.
    #[serde(default)]
    pub pre_snapshot_path: Option<String>,
    pub entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub applied: u64,
    pub skipped: u64,
    pub failed: u64,
    pub failures: Vec<String>,
    pub journal_id: Option<String>,
    #[serde(default)]
    pub creates: u64,
    #[serde(default)]
    pub overwrites: u64,
    #[serde(default)]
    pub deletes: u64,
    #[serde(default)]
    pub recovered: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPreview {
    pub creates: u64,
    pub overwrites: u64,
    pub deletes: u64,
    pub moves: u64,
    pub skips: u64,
    pub destructive: bool,
    pub rows_to_apply: u64,
}

fn journal_dir() -> PathBuf {
    codesetu_data_dir().join("journals")
}

fn backup_dir(session_id: &str) -> PathBuf {
    codesetu_data_dir().join("backups").join(session_id)
}

fn inprogress_path(session_id: &str) -> PathBuf {
    journal_dir().join(format!("{session_id}.inprogress.json"))
}

fn committed_path(session_id: &str) -> PathBuf {
    journal_dir().join(format!("{session_id}.json"))
}

fn persist_journal(journal: &Journal, path: &Path) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?;
    write_atomic(path, raw.as_bytes())
}

pub fn preview_apply(rows: &[FileRow]) -> ApplyPreview {
    let mut preview = ApplyPreview::default();
    for row in rows {
        if matches!(
            row.class,
            ChangeClass::IntranetOnly | ChangeClass::InternetOnly | ChangeClass::ParkedSecret
        ) {
            preview.skips += 1;
            continue;
        }
        match row.decision {
            Decision::Keep | Decision::Skip => {
                preview.skips += 1;
            }
            Decision::BringIn => {
                preview.rows_to_apply += 1;
                if matches!(row.class, ChangeClass::DeletedBrought) {
                    preview.deletes += 1;
                    preview.destructive = true;
                } else if row.moved_from.is_some() {
                    preview.moves += 1;
                    preview.overwrites += 1;
                    preview.destructive = true;
                } else if row.this_hash.is_some() {
                    preview.overwrites += 1;
                    preview.destructive = true;
                } else {
                    preview.creates += 1;
                }
            }
        }
    }
    preview
}

pub fn apply_plan(
    this_pc: &Path,
    brought_over: &Path,
    rows: &[FileRow],
    session_id: &str,
    snapshot: Snapshot,
    require_destructive_confirm: bool,
    confirmed_destructive: bool,
) -> Result<ApplyReport, String> {
    let preview = preview_apply(rows);
    if preview.destructive && require_destructive_confirm && !confirmed_destructive {
        return Err(format!(
            "Destructive apply requires confirmation ({} overwrites, {} deletes, {} moves).",
            preview.overwrites, preview.deletes, preview.moves
        ));
    }

    let mut report = ApplyReport {
        creates: preview.creates,
        overwrites: preview.overwrites,
        deletes: preview.deletes,
        ..Default::default()
    };

    let backups = backup_dir(session_id);
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let jdir = journal_dir();
    fs::create_dir_all(&jdir).map_err(|e| e.to_string())?;

    // Pre-apply snapshot backup (for undo + crash recovery).
    let pre_snap_path = backups.join("pre-apply-snapshot.json");
    save_snapshot_to(&pre_snap_path, &snapshot)?;

    let mut journal = Journal {
        session_id: session_id.to_string(),
        this_pc: this_pc.display().to_string(),
        brought_over: brought_over.display().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: JournalStatus::InProgress,
        pre_snapshot_path: Some(pre_snap_path.display().to_string()),
        entries: Vec::new(),
    };
    let ipath = inprogress_path(session_id);
    persist_journal(&journal, &ipath)?;

    // Working snapshot for commit — start from pre-apply; identical paths may already be present.
    let mut working = snapshot.clone();

    for row in rows {
        match row.class {
            ChangeClass::IntranetOnly | ChangeClass::InternetOnly | ChangeClass::ParkedSecret => {
                report.skipped += 1;
                continue;
            }
            _ => {}
        }

        match row.decision {
            Decision::Keep | Decision::Skip => {
                report.skipped += 1;
                if row.decision == Decision::Keep {
                    if let Some(h) = &row.this_hash {
                        working.files.insert(row.rel_path.clone(), h.clone());
                    }
                    if matches!(row.class, ChangeClass::DeletedBrought) {
                        // Persist "keep despite remote absence" so incomplete copies don't nag.
                        working.keep_despite_absent.insert(row.rel_path.clone());
                    }
                }
                continue;
            }
            Decision::BringIn => {}
        }

        let rel = match normalize_rel_path(&row.rel_path) {
            Ok(r) => r,
            Err(e) => {
                report.failed += 1;
                report.failures.push(format!("{}: {e}", row.rel_path));
                continue;
            }
        };

        if matches!(row.class, ChangeClass::DeletedBrought) {
            let dest = match safe_join(this_pc, &rel) {
                Ok(p) => p,
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("{rel}: {e}"));
                    continue;
                }
            };
            match backup_and_remove(&dest, &backups, &rel) {
                Ok(backup) => {
                    journal.entries.push(JournalEntry {
                        rel_path: rel.clone(),
                        action: "delete".into(),
                        backup_path: backup,
                        previous_hash: row.this_hash.clone(),
                        new_hash: None,
                        moved_from: None,
                        moved_from_backup: None,
                    });
                    working.files.remove(&rel);
                    working.keep_despite_absent.remove(&rel);
                    report.applied += 1;
                    let _ = persist_journal(&journal, &ipath);
                }
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("{rel}: {e}"));
                }
            }
            continue;
        }

        let src = match safe_join(brought_over, &rel) {
            Ok(p) => p,
            Err(e) => {
                report.failed += 1;
                report.failures.push(format!("{rel}: {e}"));
                continue;
            }
        };
        let dest = match safe_join(this_pc, &rel) {
            Ok(p) => p,
            Err(e) => {
                report.failed += 1;
                report.failures.push(format!("{rel}: {e}"));
                continue;
            }
        };

        // Revalidate source hash before write (TOCTOU).
        if let Some(expected) = &row.brought_hash {
            match hash_file(&src) {
                Ok((actual, _)) if &actual == expected => {}
                Ok((actual, _)) => {
                    report.failed += 1;
                    report.failures.push(format!(
                        "{rel}: source changed since scan (expected {expected}, got {actual})"
                    ));
                    continue;
                }
                Err(e) => {
                    report.failed += 1;
                    report
                        .failures
                        .push(format!("{rel}: revalidate failed: {e}"));
                    continue;
                }
            }
        }

        // If overwriting, revalidate dest hasn't drifted from scan (no silent overwrite of user edits).
        if dest.exists() {
            if let Some(expected) = &row.this_hash {
                match hash_file(&dest) {
                    Ok((actual, _)) if &actual == expected => {}
                    Ok((actual, _)) => {
                        report.failed += 1;
                        report.failures.push(format!(
                            "{rel}: local file changed since scan (expected {expected}, got {actual}) — skipped to avoid overwrite"
                        ));
                        continue;
                    }
                    Err(e) => {
                        report.failed += 1;
                        report
                            .failures
                            .push(format!("{rel}: local revalidate failed: {e}"));
                        continue;
                    }
                }
            }
        }

        let mut moved_from = None;
        let mut moved_from_backup = None;
        if let Some(from) = &row.moved_from {
            let from_norm = match normalize_rel_path(from) {
                Ok(f) => f,
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("{rel}: bad moved_from: {e}"));
                    continue;
                }
            };
            let old = match safe_join(this_pc, &from_norm) {
                Ok(p) => p,
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("{rel}: {e}"));
                    continue;
                }
            };
            match backup_and_remove(&old, &backups, &from_norm) {
                Ok(backup) => {
                    moved_from = Some(from_norm.clone());
                    moved_from_backup = backup;
                    working.files.remove(&from_norm);
                }
                Err(e) => {
                    report.failed += 1;
                    report
                        .failures
                        .push(format!("{from_norm}: move-source remove failed: {e}"));
                    continue;
                }
            }
        }

        match copy_atomic(&src, &dest, &backups, &rel) {
            Ok((backup, new_hash)) => {
                journal.entries.push(JournalEntry {
                    rel_path: rel.clone(),
                    action: if moved_from.is_some() {
                        "move".into()
                    } else {
                        "copy".into()
                    },
                    backup_path: backup,
                    previous_hash: row.this_hash.clone(),
                    new_hash: Some(new_hash.clone()),
                    moved_from,
                    moved_from_backup,
                });
                working.files.insert(rel, new_hash);
                working.keep_despite_absent.remove(&row.rel_path);
                report.applied += 1;
                let _ = persist_journal(&journal, &ipath);
            }
            Err(e) => {
                // Best-effort: if move already removed the old path, restore from its backup.
                if let (Some(from_rel), Some(bpath)) =
                    (row.moved_from.as_ref(), moved_from_backup.as_ref())
                {
                    if let Ok(old) = safe_join(this_pc, from_rel) {
                        let _ = fs::copy(bpath, old);
                    }
                }
                report.failed += 1;
                report.failures.push(format!("{rel}: {e}"));
            }
        }
    }

    // Commit snapshot only after mutations + durable journal.
    working.this_pc = this_pc.display().to_string();
    working.brought_over = brought_over.display().to_string();
    working.updated_at = chrono::Utc::now().to_rfc3339();
    working.version = 2;
    working.generation = snapshot.generation.saturating_add(1);
    save_snapshot(&working)?;

    journal.status = JournalStatus::Committed;
    let cpath = committed_path(session_id);
    persist_journal(&journal, &cpath)?;
    let _ = fs::remove_file(&ipath);
    let latest = journal_dir().join("latest.txt");
    write_atomic(&latest, session_id.as_bytes())?;
    report.journal_id = Some(session_id.to_string());
    // silence unused
    let _ = snapshot;
    Ok(report)
}

/// Detect incomplete journals and roll them back (files + snapshot).
pub fn recover_incomplete() -> Result<Vec<ApplyReport>, String> {
    let jdir = journal_dir();
    if !jdir.exists() {
        return Ok(vec![]);
    }
    let mut reports = Vec::new();
    for entry in fs::read_dir(&jdir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".inprogress.json") {
            continue;
        }
        let raw = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        let journal: Journal = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let mut report = rollback_journal(&journal)?;
        report.recovered = true;
        report.message = Some(format!(
            "Rolled back incomplete apply {}",
            journal.session_id
        ));
        let _ = fs::remove_file(entry.path());
        reports.push(report);
    }
    Ok(reports)
}

pub fn undo_last() -> Result<ApplyReport, String> {
    // Prefer recovering incomplete first.
    let recovered = recover_incomplete()?;
    if let Some(r) = recovered.into_iter().next() {
        return Ok(r);
    }

    let jdir = journal_dir();
    let latest =
        fs::read_to_string(jdir.join("latest.txt")).map_err(|_| "Nothing to undo.".to_string())?;
    let session_id = latest.trim().to_string();
    let jpath = committed_path(&session_id);
    let raw = fs::read_to_string(&jpath).map_err(|e| e.to_string())?;
    let journal: Journal = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let report = rollback_journal(&journal)?;
    let _ = fs::remove_file(jdir.join("latest.txt"));
    // Mark journal rolled back
    let mut marked = journal;
    marked.status = JournalStatus::RolledBack;
    let _ = persist_journal(&marked, &jpath);
    Ok(report)
}

fn rollback_journal(journal: &Journal) -> Result<ApplyReport, String> {
    let this_pc = PathBuf::from(&journal.this_pc);
    let mut report = ApplyReport::default();

    for entry in journal.entries.iter().rev() {
        let dest = match safe_join(&this_pc, &entry.rel_path) {
            Ok(p) => p,
            Err(e) => {
                report.failed += 1;
                report.failures.push(format!("{}: {e}", entry.rel_path));
                continue;
            }
        };
        match entry.action.as_str() {
            "copy" | "move" => {
                if let Some(backup) = &entry.backup_path {
                    match fs::copy(backup, &dest) {
                        Ok(_) => report.applied += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            let _ = fs::remove_file(&dest);
                            report.applied += 1;
                        }
                        Err(e) => {
                            report.failed += 1;
                            report.failures.push(format!("{}: {e}", entry.rel_path));
                        }
                    }
                } else {
                    match fs::remove_file(&dest) {
                        Ok(_) => report.applied += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => report.applied += 1,
                        Err(e) => {
                            report.failed += 1;
                            report.failures.push(format!("{}: {e}", entry.rel_path));
                        }
                    }
                }
                // Restore moved-from path
                if let (Some(from), Some(backup)) = (&entry.moved_from, &entry.moved_from_backup) {
                    if let Ok(old) = safe_join(&this_pc, from) {
                        if let Some(parent) = old.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        match fs::copy(backup, &old) {
                            Ok(_) => report.applied += 1,
                            Err(e) => {
                                report.failed += 1;
                                report
                                    .failures
                                    .push(format!("{from}: restore move source: {e}"));
                            }
                        }
                    }
                } else if let Some(from) = &entry.moved_from {
                    // No backup recorded (shouldn't happen) — still try nothing
                    let _ = from;
                }
            }
            "delete" => {
                if let Some(backup) = &entry.backup_path {
                    if let Some(parent) = dest.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    match fs::copy(backup, &dest) {
                        Ok(_) => report.applied += 1,
                        Err(e) => {
                            report.failed += 1;
                            report.failures.push(format!("{}: {e}", entry.rel_path));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Restore pre-apply snapshot
    if let Some(pre) = &journal.pre_snapshot_path {
        let pre_path = Path::new(pre);
        if pre_path.exists() {
            match load_snapshot_from(pre_path) {
                Ok(snap) => {
                    if let Err(e) = save_snapshot(&snap) {
                        report.failed += 1;
                        report
                            .failures
                            .push(format!("snapshot restore failed: {e}"));
                    }
                }
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("snapshot load failed: {e}"));
                }
            }
        }
    }

    report.journal_id = Some(journal.session_id.clone());
    Ok(report)
}

fn backup_and_remove(dest: &Path, backups: &Path, rel: &str) -> Result<Option<String>, String> {
    if !dest.exists() {
        return Ok(None);
    }
    let backup = backups.join(rel);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(dest, &backup).map_err(|e| e.to_string())?;
    fsync_path(&backup).ok();
    remove_file_resilient(dest)?;
    Ok(Some(backup.display().to_string()))
}

fn copy_atomic(
    src: &Path,
    dest: &Path,
    backups: &Path,
    rel: &str,
) -> Result<(Option<String>, String), String> {
    if !src.exists() {
        return Err(format!("source missing: {}", src.display()));
    }
    let mut backup = None;
    if dest.exists() {
        let b = backups.join(rel);
        if let Some(parent) = b.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(dest, &b).map_err(|e| e.to_string())?;
        fsync_path(&b).ok();
        backup = Some(b.display().to_string());
        // Keep dest in place until replace — Windows MoveFileEx replaces without
        // a prior delete (delete-first races AV and yields Access Denied).
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Prefer shared helper (temp + fsync + durable replace).
    replace_file(src, dest).inspect_err(|_e| {
        // Best-effort temp cleanup if replace_file left one behind.
        let tmp = dest.with_file_name(format!(
            "{}.codesetu.tmp",
            dest.file_name().and_then(|s| s.to_str()).unwrap_or("file")
        ));
        let _ = remove_file_resilient(&tmp);
    })?;
    let (hash, _) = hash_file(dest)?;
    Ok((backup, hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{FileKind, PathZone};
    use tempfile::tempdir;

    fn row(
        rel: &str,
        class: ChangeClass,
        decision: Decision,
        this_hash: Option<&str>,
        brought_hash: Option<&str>,
        moved_from: Option<&str>,
    ) -> FileRow {
        FileRow {
            id: rel.into(),
            rel_path: rel.into(),
            group: "src".into(),
            class,
            decision,
            zone: PathZone::Shared,
            kind: FileKind::Text,
            this_size: None,
            brought_size: None,
            this_hash: this_hash.map(|s| s.to_string()),
            brought_hash: brought_hash.map(|s| s.to_string()),
            moved_from: moved_from.map(|s| s.to_string()),
            error: None,
        }
    }

    fn with_data_dir<T>(f: impl FnOnce(&Path) -> T) -> T {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(dir.path(), || f(dir.path()))
    }

    #[test]
    fn copies_bring_in_and_skips_intranet_only() {
        with_data_dir(|data| {
            let this_pc = data.join("intra");
            let brought = data.join("inet");
            fs::create_dir_all(this_pc.join("src")).unwrap();
            fs::create_dir_all(brought.join("src")).unwrap();
            fs::create_dir_all(this_pc.join("deploy/intra")).unwrap();
            fs::create_dir_all(brought.join("deploy/intra")).unwrap();
            fs::write(this_pc.join("src/a.txt"), b"old").unwrap();
            fs::write(brought.join("src/a.txt"), b"new").unwrap();
            fs::write(this_pc.join("deploy/intra/x.yml"), b"keep-me").unwrap();
            fs::write(brought.join("deploy/intra/x.yml"), b"ignore").unwrap();

            let (old_hash, _) = hash_file(&this_pc.join("src/a.txt")).unwrap();
            let (new_hash, _) = hash_file(&brought.join("src/a.txt")).unwrap();

            let rows = vec![
                row(
                    "src/a.txt",
                    ChangeClass::BringIn,
                    Decision::BringIn,
                    Some(&old_hash),
                    Some(&new_hash),
                    None,
                ),
                FileRow {
                    id: "2".into(),
                    rel_path: "deploy/intra/x.yml".into(),
                    group: "deploy".into(),
                    class: ChangeClass::IntranetOnly,
                    decision: Decision::Keep,
                    zone: PathZone::IntranetOnly,
                    kind: FileKind::Text,
                    this_size: Some(7),
                    brought_size: Some(6),
                    this_hash: None,
                    brought_hash: None,
                    moved_from: None,
                    error: None,
                },
            ];

            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            let report = apply_plan(
                &this_pc,
                &brought,
                &rows,
                "test-session",
                snap,
                false,
                false,
            )
            .unwrap();
            assert_eq!(report.applied, 1);
            assert_eq!(
                fs::read_to_string(this_pc.join("src/a.txt")).unwrap(),
                "new"
            );
            assert_eq!(
                fs::read_to_string(this_pc.join("deploy/intra/x.yml")).unwrap(),
                "keep-me"
            );
        });
    }

    #[test]
    fn overwrites_existing_file() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            fs::write(this_pc.join("f.txt"), b"old").unwrap();
            fs::write(brought.join("f.txt"), b"new-content").unwrap();
            let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
            let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();
            let rows = vec![row(
                "f.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&th),
                Some(&bh),
                None,
            )];
            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            apply_plan(&this_pc, &brought, &rows, "ow", snap, false, false).unwrap();
            assert_eq!(
                fs::read_to_string(this_pc.join("f.txt")).unwrap(),
                "new-content"
            );
        });
    }

    #[test]
    fn undo_restores_files_and_snapshot() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            fs::write(this_pc.join("f.txt"), b"local").unwrap();
            fs::write(brought.join("f.txt"), b"remote").unwrap();
            let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
            let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();

            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            snap.files.insert("f.txt".into(), th.clone());
            snap.generation = 1;
            save_snapshot(&snap).unwrap();

            let rows = vec![row(
                "f.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&th),
                Some(&bh),
                None,
            )];
            apply_plan(
                &this_pc,
                &brought,
                &rows,
                "undo-sess",
                snap.clone(),
                false,
                false,
            )
            .unwrap();
            assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "remote");

            let after = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
            assert_eq!(after.files.get("f.txt").unwrap(), &bh);

            undo_last().unwrap();
            assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "local");
            let restored = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
            assert_eq!(restored.files.get("f.txt").unwrap(), &th);
            assert_eq!(restored.generation, 1);
        });
    }

    #[test]
    fn move_undo_restores_both_paths() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(this_pc.join("old")).unwrap();
            fs::create_dir_all(brought.join("new")).unwrap();
            fs::write(this_pc.join("old/a.txt"), b"same-bytes").unwrap();
            fs::write(brought.join("new/a.txt"), b"same-bytes").unwrap();
            let (h, _) = hash_file(&brought.join("new/a.txt")).unwrap();
            let rows = vec![row(
                "new/a.txt",
                ChangeClass::Moved,
                Decision::BringIn,
                None,
                Some(&h),
                Some("old/a.txt"),
            )];
            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            snap.files.insert("old/a.txt".into(), h.clone());
            apply_plan(&this_pc, &brought, &rows, "move-sess", snap, false, false).unwrap();
            assert!(!this_pc.join("old/a.txt").exists());
            assert!(this_pc.join("new/a.txt").exists());
            undo_last().unwrap();
            assert!(this_pc.join("old/a.txt").exists());
            assert!(!this_pc.join("new/a.txt").exists());
        });
    }

    #[test]
    fn crash_mid_apply_recovers() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            fs::write(this_pc.join("a.txt"), b"A0").unwrap();
            fs::write(brought.join("a.txt"), b"A1").unwrap();
            let (th, _) = hash_file(&this_pc.join("a.txt")).unwrap();
            let (bh, _) = hash_file(&brought.join("a.txt")).unwrap();

            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            snap.files.insert("a.txt".into(), th.clone());
            save_snapshot(&snap).unwrap();

            // Simulate partial apply: write inprogress journal with one entry already applied.
            let sid = "crash-sess";
            let backups = backup_dir(sid);
            fs::create_dir_all(&backups).unwrap();
            let pre = backups.join("pre-apply-snapshot.json");
            save_snapshot_to(&pre, &snap).unwrap();
            let backup = backups.join("a.txt");
            fs::copy(this_pc.join("a.txt"), &backup).unwrap();
            fs::write(this_pc.join("a.txt"), b"A1").unwrap();

            let journal = Journal {
                session_id: sid.into(),
                this_pc: this_pc.display().to_string(),
                brought_over: brought.display().to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                status: JournalStatus::InProgress,
                pre_snapshot_path: Some(pre.display().to_string()),
                entries: vec![JournalEntry {
                    rel_path: "a.txt".into(),
                    action: "copy".into(),
                    backup_path: Some(backup.display().to_string()),
                    previous_hash: Some(th.clone()),
                    new_hash: Some(bh),
                    moved_from: None,
                    moved_from_backup: None,
                }],
            };
            fs::create_dir_all(journal_dir()).unwrap();
            persist_journal(&journal, &inprogress_path(sid)).unwrap();

            let reports = recover_incomplete().unwrap();
            assert_eq!(reports.len(), 1);
            assert_eq!(fs::read_to_string(this_pc.join("a.txt")).unwrap(), "A0");
            let restored = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
            assert_eq!(restored.files.get("a.txt").unwrap(), &th);
        });
    }

    #[test]
    fn rejects_changed_source_on_revalidate() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            fs::write(this_pc.join("f.txt"), b"local").unwrap();
            fs::write(brought.join("f.txt"), b"remote-now").unwrap();
            let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
            let rows = vec![row(
                "f.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&th),
                Some("deadbeef"),
                None,
            )];
            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            let report =
                apply_plan(&this_pc, &brought, &rows, "reval", snap, false, false).unwrap();
            assert_eq!(report.failed, 1);
            assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "local");
        });
    }

    #[test]
    fn path_traversal_rejected() {
        with_data_dir(|data| {
            let this_pc = data.join("intra");
            let brought = data.join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            let rows = vec![row(
                "../escape.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                None,
                None,
                None,
            )];
            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            let report = apply_plan(&this_pc, &brought, &rows, "trav", snap, false, false).unwrap();
            assert_eq!(report.failed, 1);
        });
    }

    #[test]
    fn destructive_confirm_required() {
        with_data_dir(|_| {
            let dir = tempdir().unwrap();
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            fs::write(this_pc.join("f.txt"), b"x").unwrap();
            fs::write(brought.join("f.txt"), b"y").unwrap();
            let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
            let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();
            let rows = vec![row(
                "f.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&th),
                Some(&bh),
                None,
            )];
            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            let err = apply_plan(&this_pc, &brought, &rows, "conf", snap, true, false).unwrap_err();
            assert!(err.contains("confirmation"));
        });
    }
}
