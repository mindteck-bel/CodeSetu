//! Persistent local synchronization history (offline, capped).
//!
//! Records apply / undo / recover outcomes under the CodeSetu data directory.
//! Never leaves the machine; no network.

use crate::engine::paths::codesetu_data_dir;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::OnceLock;

const MAX_ENTRIES: usize = 200;

fn history_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn history_path() -> std::path::PathBuf {
    codesetu_data_dir()
        .join("history")
        .join("sync-history.jsonl")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub ts: String,
    pub kind: String,
    pub this_pc: Option<String>,
    pub brought_over: Option<String>,
    pub journal_id: Option<String>,
    pub applied: u64,
    pub kept: u64,
    pub skipped: u64,
    pub failed: u64,
    pub deletes: u64,
    pub message: Option<String>,
}

/// Append one history row. Best-effort; never panics callers.
pub fn record_event(entry: &HistoryEntry) {
    let _guard = history_lock().lock();
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = match serde_json::to_string(entry) {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
        let _ = f.sync_all();
    }
}

pub fn record_from_report(
    kind: &str,
    this_pc: Option<&str>,
    brought_over: Option<&str>,
    report: &crate::engine::ApplyReport,
) {
    record_event(&HistoryEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        kind: kind.to_string(),
        this_pc: this_pc.map(|s| s.to_string()),
        brought_over: brought_over.map(|s| s.to_string()),
        journal_id: report.journal_id.clone(),
        applied: report.applied,
        kept: report.kept,
        skipped: report.skipped,
        failed: report.failed,
        deletes: report.deletes,
        message: report.message.clone(),
    });
}

/// Newest-first, capped list for UI / diagnostics.
pub fn list_recent(limit: usize) -> Vec<HistoryEntry> {
    let _guard = history_lock().lock();
    let path = history_path();
    let Ok(file) = fs::File::open(&path) else {
        return vec![];
    };
    let mut entries = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(trimmed) {
            entries.push(entry);
        }
    }
    let cap = limit.clamp(1, MAX_ENTRIES);
    let recent = if entries.len() > cap {
        entries.split_off(entries.len() - cap)
    } else {
        entries
    };
    recent.into_iter().rev().collect()
}

/// Compact JSON array for diagnostic export (newest-first).
pub fn recent_as_json(limit: usize) -> serde_json::Value {
    json!(list_recent(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::paths::with_test_data_dir;
    use tempfile::tempdir;

    #[test]
    fn records_and_lists_newest_first() {
        let dir = tempdir().unwrap();
        with_test_data_dir(dir.path(), || {
            record_event(&HistoryEntry {
                ts: "2026-01-01T00:00:00Z".into(),
                kind: "apply".into(),
                this_pc: Some("/a".into()),
                brought_over: Some("/b".into()),
                journal_id: Some("j1".into()),
                applied: 2,
                kept: 0,
                skipped: 0,
                failed: 0,
                deletes: 0,
                message: Some("ok".into()),
            });
            record_event(&HistoryEntry {
                ts: "2026-01-02T00:00:00Z".into(),
                kind: "undo".into(),
                this_pc: None,
                brought_over: None,
                journal_id: Some("j1".into()),
                applied: 2,
                kept: 0,
                skipped: 0,
                failed: 0,
                deletes: 0,
                message: None,
            });
            let recent = list_recent(10);
            assert_eq!(recent.len(), 2);
            assert_eq!(recent[0].kind, "undo");
            assert_eq!(recent[1].kind, "apply");
        });
    }
}
