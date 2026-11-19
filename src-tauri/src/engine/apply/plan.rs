//! Journal-first apply loop (recoverable batch, not multi-file atomic).

use super::journal::{
    backup_dir, committed_path, inprogress_path, journal_dir, persist_journal, Journal,
    JournalEntry, JournalStatus,
};
use super::ops::{backup_and_remove, copy_atomic};
use super::preview::preview_apply;
use super::report::ApplyReport;
use crate::engine::diag;
use crate::engine::fsutil::write_atomic;
use crate::engine::hash::hash_file;
use crate::engine::paths::{normalize_rel_path, safe_join};
use crate::engine::snapshot::{save_snapshot, save_snapshot_to, Snapshot};
use crate::engine::types::{ChangeClass, Decision, FileRow};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn apply_plan(
    this_pc: &Path,
    brought_over: &Path,
    rows: &[FileRow],
    session_id: &str,
    snapshot: Snapshot,
    require_destructive_confirm: bool,
    confirmed_destructive: bool,
    only_ids: Option<&[String]>,
) -> Result<ApplyReport, String> {
    let first_sync = snapshot.generation == 0 && snapshot.files.is_empty();
    let preview = preview_apply(rows, first_sync, only_ids);
    let id_filter: Option<HashSet<&str>> =
        only_ids.map(|ids| ids.iter().map(|s| s.as_str()).collect());
    if preview.rows_to_apply == 0 && preview.keeps == 0 {
        let scope = if preview.selected_count > 0 {
            "selected file(s)"
        } else {
            "files"
        };
        return Err(format!(
            "Nothing to apply — choose Use source or Keep mine on the {scope} first. Unresolved Skip leaves the destination unchanged."
        ));
    }
    if preview.destructive && require_destructive_confirm && !confirmed_destructive {
        return Err(format!(
            "Destructive apply requires confirmation ({} overwrites, {} deletes, {} moves).",
            preview.overwrites, preview.deletes, preview.moves
        ));
    }
    if preview.first_sync
        && preview.creates > 0
        && require_destructive_confirm
        && !confirmed_destructive
    {
        return Err(format!(
            "First sync requires confirmation before creating {} new file(s) on Intranet.",
            preview.creates
        ));
    }

    diag::log_event(
        "apply.start",
        json!({
            "sessionId": session_id,
            "firstSync": preview.first_sync,
            "creates": preview.creates,
            "overwrites": preview.overwrites,
            "deletes": preview.deletes,
            "moves": preview.moves,
        }),
    );

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
        if let Some(ref allowed) = id_filter {
            if !allowed.contains(row.id.as_str()) {
                // Out of selective-apply scope — leave file and snapshot entry untouched.
                continue;
            }
        }
        match row.class {
            ChangeClass::IntranetOnly | ChangeClass::InternetOnly | ChangeClass::ParkedSecret => {
                report.skipped += 1;
                continue;
            }
            _ => {}
        }

        match row.decision {
            Decision::Keep => {
                // Keep mine: leave destination bytes alone; accept current baseline.
                report.kept += 1;
                if let Some(h) = &row.this_hash {
                    working.files.insert(row.rel_path.clone(), h.clone());
                }
                if matches!(row.class, ChangeClass::DeletedBrought) {
                    // Persist "keep despite remote absence" so incomplete copies don't nag.
                    working.keep_despite_absent.insert(row.rel_path.clone());
                }
                continue;
            }
            Decision::Skip => {
                report.skipped += 1;
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
    let _ = snapshot;
    // Plain local-folder summary — never Git push/pull language.
    if report.message.is_none() {
        report.message = Some(summarize_apply(&report));
    }
    diag::log_event(
        "apply.commit",
        json!({
            "sessionId": session_id,
            "applied": report.applied,
            "kept": report.kept,
            "failed": report.failed,
            "skipped": report.skipped,
        }),
    );
    crate::engine::history::record_from_report(
        "apply",
        Some(&this_pc.display().to_string()),
        Some(&brought_over.display().to_string()),
        &report,
    );
    Ok(report)
}

fn plural(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}

fn summarize_apply(report: &ApplyReport) -> String {
    let mut parts = Vec::new();
    if report.applied > 0 {
        parts.push(format!(
            "{} updated on destination",
            plural(report.applied, "file", "files")
        ));
    }
    if report.kept > 0 {
        parts.push(format!(
            "{} kept as mine (destination unchanged)",
            report.kept
        ));
    }
    if report.skipped > 0 {
        parts.push(format!(
            "{} left unresolved (Skip) — destination not changed",
            report.skipped
        ));
    }
    if report.failed > 0 {
        parts.push(format!("{} failed", report.failed));
    }
    if parts.is_empty() {
        "Nothing changed on the destination folder.".into()
    } else if report.applied == 0 && report.failed == 0 {
        format!("Nothing written to destination · {}", parts.join(" · "))
    } else {
        parts.join(" · ")
    }
}
