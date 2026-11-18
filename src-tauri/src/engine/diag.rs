//! Local offline diagnostic log (JSONL under the CodeSetu data directory).
//! Never leaves the machine; no network.

use crate::engine::paths::codesetu_data_dir;
use parking_lot::Mutex;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
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
}
