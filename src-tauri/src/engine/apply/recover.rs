//! Crash recovery and undo (restores files + pre-apply snapshot).

use super::journal::{committed_path, journal_dir, persist_journal, Journal, JournalStatus};
use super::report::ApplyReport;
use crate::engine::diag;
use crate::engine::paths::safe_join;
use crate::engine::snapshot::{load_snapshot_from, save_snapshot};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

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
        diag::log_warn(
            "apply.recover",
            format!("Rolled back incomplete journal {}", journal.session_id),
        );
        crate::engine::history::record_from_report(
            "recover",
            Some(&journal.this_pc),
            Some(&journal.brought_over),
            &report,
        );
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
    diag::log_event(
        "apply.undo",
        json!({
            "sessionId": session_id,
            "applied": report.applied,
            "failed": report.failed,
        }),
    );
    crate::engine::history::record_from_report(
        "undo",
        Some(&marked.this_pc),
        Some(&marked.brought_over),
        &report,
    );
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
