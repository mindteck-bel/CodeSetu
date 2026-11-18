use crate::engine::snapshot::{save_snapshot, Snapshot};
use crate::engine::types::{ChangeClass, Decision, FileRow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntry {
    pub rel_path: String,
    pub action: String,
    pub backup_path: Option<String>,
    pub previous_hash: Option<String>,
    pub new_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    pub session_id: String,
    pub this_pc: String,
    pub brought_over: String,
    pub created_at: String,
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
}

fn journal_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("CodeSetu")
        .join("journals")
}

fn backup_dir(session_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("CodeSetu")
        .join("backups")
        .join(session_id)
}

pub fn apply_plan(
    this_pc: &Path,
    brought_over: &Path,
    rows: &[FileRow],
    session_id: &str,
    mut snapshot: Snapshot,
) -> Result<ApplyReport, String> {
    let mut report = ApplyReport::default();
    let mut journal = Journal {
        session_id: session_id.to_string(),
        this_pc: this_pc.display().to_string(),
        brought_over: brought_over.display().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        entries: Vec::new(),
    };

    let backups = backup_dir(session_id);
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;

    for row in rows {
        // Never write env-only / internet-only / parked unless explicitly BringIn on shared/moved
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
                        snapshot.files.insert(row.rel_path.clone(), h.clone());
                    }
                }
                continue;
            }
            Decision::BringIn => {}
        }

        if matches!(row.class, ChangeClass::DeletedBrought) {
            // delete local file
            let dest = this_pc.join(&row.rel_path);
            match backup_and_remove(&dest, &backups, &row.rel_path) {
                Ok(backup) => {
                    journal.entries.push(JournalEntry {
                        rel_path: row.rel_path.clone(),
                        action: "delete".into(),
                        backup_path: backup,
                        previous_hash: row.this_hash.clone(),
                        new_hash: None,
                    });
                    snapshot.files.remove(&row.rel_path);
                    report.applied += 1;
                }
                Err(e) => {
                    report.failed += 1;
                    report.failures.push(format!("{}: {e}", row.rel_path));
                }
            }
            continue;
        }

        let src = if let Some(from) = &row.moved_from {
            // move: source content is at brought new path
            brought_over.join(&row.rel_path)
        } else {
            brought_over.join(&row.rel_path)
        };
        let dest = this_pc.join(&row.rel_path);

        if let Some(from) = &row.moved_from {
            let old = this_pc.join(from);
            let _ = backup_and_remove(&old, &backups, from);
            snapshot.files.remove(from);
        }

        match copy_atomic(&src, &dest, &backups, &row.rel_path) {
            Ok((backup, new_hash)) => {
                journal.entries.push(JournalEntry {
                    rel_path: row.rel_path.clone(),
                    action: "copy".into(),
                    backup_path: backup,
                    previous_hash: row.this_hash.clone(),
                    new_hash: Some(new_hash.clone()),
                });
                snapshot.files.insert(row.rel_path.clone(), new_hash);
                report.applied += 1;
            }
            Err(e) => {
                report.failed += 1;
                report.failures.push(format!("{}: {e}", row.rel_path));
            }
        }
    }

    snapshot.this_pc = this_pc.display().to_string();
    snapshot.brought_over = brought_over.display().to_string();
    snapshot.updated_at = chrono::Utc::now().to_rfc3339();
    snapshot.version = 1;
    save_snapshot(&snapshot)?;

    let jdir = journal_dir();
    fs::create_dir_all(&jdir).map_err(|e| e.to_string())?;
    let jpath = jdir.join(format!("{session_id}.json"));
    let tmp = jpath.with_extension("json.tmp");
    let raw = serde_json::to_string_pretty(&journal).map_err(|e| e.to_string())?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(raw.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, &jpath).map_err(|e| e.to_string())?;
    // pointer to latest
    let latest = jdir.join("latest.txt");
    fs::write(&latest, session_id).ok();
    report.journal_id = Some(session_id.to_string());
    Ok(report)
}

pub fn undo_last() -> Result<ApplyReport, String> {
    let jdir = journal_dir();
    let latest = fs::read_to_string(jdir.join("latest.txt")).map_err(|_| {
        "Nothing to undo.".to_string()
    })?;
    let session_id = latest.trim().to_string();
    let jpath = jdir.join(format!("{session_id}.json"));
    let raw = fs::read_to_string(&jpath).map_err(|e| e.to_string())?;
    let journal: Journal = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let this_pc = PathBuf::from(&journal.this_pc);

    let mut report = ApplyReport::default();
    for entry in journal.entries.iter().rev() {
        let dest = this_pc.join(&entry.rel_path);
        match entry.action.as_str() {
            "copy" => {
                if let Some(backup) = &entry.backup_path {
                    match fs::copy(backup, &dest) {
                        Ok(_) => report.applied += 1,
                        Err(e) => {
                            // backup missing means file was new — remove it
                            if e.kind() == std::io::ErrorKind::NotFound {
                                let _ = fs::remove_file(&dest);
                                report.applied += 1;
                            } else {
                                report.failed += 1;
                                report.failures.push(format!("{}: {e}", entry.rel_path));
                            }
                        }
                    }
                } else {
                    // was a new file
                    match fs::remove_file(&dest) {
                        Ok(_) => report.applied += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => report.applied += 1,
                        Err(e) => {
                            report.failed += 1;
                            report.failures.push(format!("{}: {e}", entry.rel_path));
                        }
                    }
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

    let _ = fs::remove_file(jdir.join("latest.txt"));
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
    fs::remove_file(dest).map_err(|e| e.to_string())?;
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
        backup = Some(b.display().to_string());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension(format!(
        "{}.codesetu.tmp",
        dest.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("part")
    ));
    fs::copy(src, &tmp).map_err(|e| e.to_string())?;
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    let (hash, _) = crate::engine::hash::hash_file(dest)?;
    Ok((backup, hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{FileKind, PathZone};
    use tempfile::tempdir;

    #[test]
    fn copies_bring_in_and_skips_intranet_only() {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(this_pc.join("src")).unwrap();
        fs::create_dir_all(brought.join("src")).unwrap();
        fs::create_dir_all(this_pc.join("deploy/intra")).unwrap();
        fs::write(this_pc.join("src/a.txt"), b"old").unwrap();
        fs::write(brought.join("src/a.txt"), b"new").unwrap();
        fs::write(this_pc.join("deploy/intra/x.yml"), b"keep-me").unwrap();
        fs::write(brought.join("deploy/intra/x.yml"), b"ignore").unwrap();

        let rows = vec![
            FileRow {
                id: "1".into(),
                rel_path: "src/a.txt".into(),
                group: "src".into(),
                class: ChangeClass::BringIn,
                decision: Decision::BringIn,
                zone: PathZone::Shared,
                kind: FileKind::Text,
                this_size: Some(3),
                brought_size: Some(3),
                this_hash: None,
                brought_hash: None,
                moved_from: None,
                error: None,
            },
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

        let report = apply_plan(&this_pc, &brought, &rows, "test-session", Snapshot::default())
            .unwrap();
        assert_eq!(report.applied, 1);
        assert_eq!(fs::read_to_string(this_pc.join("src/a.txt")).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(this_pc.join("deploy/intra/x.yml")).unwrap(),
            "keep-me"
        );
    }
}
