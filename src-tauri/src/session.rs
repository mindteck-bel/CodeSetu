//! In-memory scan/apply session state isolated from the thin command layer.

use crate::engine::FileRow;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AppState {
    pub cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub sessions: Mutex<HashMap<String, SessionData>>,
    /// Serialize apply / undo across the process.
    pub apply_lock: Mutex<()>,
    /// Bump when ignore rules change so UI must rescan.
    pub ignore_epoch: Mutex<u64>,
}

pub struct SessionData {
    pub this_pc: PathBuf,
    pub brought_over: PathBuf,
    pub rows: Vec<FileRow>,
    pub ignore_epoch: u64,
    /// Empty baseline for this folder pair — first sync safety gate.
    pub first_sync: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            apply_lock: Mutex::new(()),
            ignore_epoch: Mutex::new(0),
        }
    }
}

impl AppState {
    pub fn bump_ignore_epoch(&self) {
        *self.ignore_epoch.lock() += 1;
    }

    pub fn ignore_epoch(&self) -> u64 {
        *self.ignore_epoch.lock()
    }
}

pub fn ensure_ignore_fresh(session: &SessionData, epoch: u64) -> Result<(), String> {
    if session.ignore_epoch != epoch {
        return Err(
            "Ignore rules changed since this scan. Run a fresh compare before applying.".into(),
        );
    }
    Ok(())
}
