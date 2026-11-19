//! Local offline diagnostic log (JSONL under the CodeSetu data directory).
//! Never leaves the machine; no network.
//!
//! Phase 2: [`export_diagnostic_report`] bundles logs + sync history + journal
//! status into a single JSON file the pilot can copy off-box manually.

use crate::engine::history;
use crate::engine::paths::codesetu_data_dir;
use parking_lot::Mutex;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn log_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn log_path() -> std::path::PathBuf {
    codesetu_data_dir().join("logs").join("codesetu.jsonl")
}

/// Append one diagnostic event. Best-effort; never panics callers.
pub fn log_event(kind: &str, fields: serde_json::Value) {
    let _guard = log_lock().lock();
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut obj = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "kind": kind,
    });
    if let Some(map) = obj.as_object_mut() {
        if let Some(extra) = fields.as_object() {
            for (k, v) in extra {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    let line = match serde_json::to_string(&obj) {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
        let _ = f.sync_all();
    }
}

pub fn log_info(kind: &str, message: impl AsRef<str>) {
    log_event(
        kind,
        json!({ "level": "info", "message": message.as_ref() }),
    );
}

pub fn log_warn(kind: &str, message: impl AsRef<str>) {
    log_event(
        kind,
        json!({ "level": "warn", "message": message.as_ref() }),
    );
}

pub fn log_error(kind: &str, message: impl AsRef<str>) {
    log_event(
        kind,
        json!({ "level": "error", "message": message.as_ref() }),
    );
}

fn read_recent_log_lines(max: usize) -> Vec<serde_json::Value> {
    let Ok(file) = fs::File::open(log_path()) else {
        return vec![];
    };
    let mut lines: Vec<String> = BufReader::new(file).lines().map_while(Result::ok).collect();
    if lines.len() > max {
        lines = lines.split_off(lines.len() - max);
    }
    lines
        .into_iter()
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

fn journal_status_summary() -> serde_json::Value {
    let jdir = codesetu_data_dir().join("journals");
    let mut in_progress = Vec::new();
    let mut committed = 0u64;
    if let Ok(rd) = fs::read_dir(&jdir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".inprogress.json") {
                in_progress.push(name);
            } else if name.ends_with(".json") {
                committed += 1;
            }
        }
    }
    let latest = fs::read_to_string(jdir.join("latest.txt"))
        .ok()
        .map(|s| s.trim().to_string());
    json!({
        "inProgress": in_progress,
        "committedCount": committed,
        "latestJournalId": latest,
    })
}

/// Write a self-contained diagnostic JSON report to `dest`.
/// Paths are included as-is (local machine). No network; caller copies the file.
pub fn export_diagnostic_report(dest: &Path) -> Result<PathBuf, String> {
    if dest.as_os_str().is_empty() {
        return Err("Choose a destination file for the diagnostic report.".into());
    }
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Cannot create folder: {e}"))?;
        }
    }
    let report = json!({
        "product": "CodeSetu",
        "reportKind": "pilot-diagnostics",
        "generatedAt": chrono::Utc::now().to_rfc3339(),
        "appVersion": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "dataDir": codesetu_data_dir().display().to_string(),
        "notes": [
            "This report stays on the machine until you copy it. CodeSetu never uploads it.",
            "Share only with your pilot contact. Paths may reveal project locations.",
            "No secrets are intentionally collected; still review before sharing off-network."
        ],
        "journals": journal_status_summary(),
        "syncHistory": history::recent_as_json(50),
        "recentLogEvents": read_recent_log_lines(200),
    });
    let pretty = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("json.tmp");
    fs::write(&tmp, pretty.as_bytes()).map_err(|e| format!("Cannot write report: {e}"))?;
    fs::rename(&tmp, dest).map_err(|e| format!("Cannot finalize report: {e}"))?;
    log_info(
        "diag.export",
        format!("Exported diagnostic report to {}", dest.display()),
    );
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::paths::with_test_data_dir;
    use tempfile::tempdir;

    #[test]
    fn writes_jsonl_line() {
        let dir = tempdir().unwrap();
        with_test_data_dir(dir.path(), || {
            log_info("test.event", "hello");
            let raw = fs::read_to_string(log_path()).unwrap();
            assert!(raw.contains("test.event"));
            assert!(raw.contains("hello"));
        });
    }

    #[test]
    fn exports_diagnostic_report() {
        let dir = tempdir().unwrap();
        with_test_data_dir(dir.path(), || {
            log_warn("test.warn", "pilot");
            history::record_event(&history::HistoryEntry {
                ts: "2026-01-01T00:00:00Z".into(),
                kind: "apply".into(),
                this_pc: Some("/a".into()),
                brought_over: Some("/b".into()),
                journal_id: Some("j".into()),
                applied: 1,
                kept: 0,
                skipped: 0,
                failed: 0,
                deletes: 0,
                message: None,
            });
            let dest = dir.path().join("out").join("codesetu-diag.json");
            let written = export_diagnostic_report(&dest).unwrap();
            assert_eq!(written, dest);
            let raw = fs::read_to_string(&dest).unwrap();
            assert!(raw.contains("pilot-diagnostics"));
            assert!(raw.contains("test.warn"));
            assert!(raw.contains("\"kind\": \"apply\"") || raw.contains("\"kind\":\"apply\""));
        });
    }
}
