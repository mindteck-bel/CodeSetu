//! CodeSetu sync engine — pure-ish filesystem logic behind the Tauri command layer.
//!
//! Public surface:
//! - [`classify`] / [`scan_tree`] — detect changes
//! - [`apply_plan`] / [`undo_last`] / [`recover_incomplete`] — mutate Intranet safely
//! - [`SyncRules`] / [`IgnoreRulesView`] — ignore + zone rules (`codesetu.toml` compatible)
//! - [`load_snapshot`] / [`save_snapshot`] — local baselines
//! - [`paths`] — path confinement
//! - [`diag`] — offline diagnostic JSONL
//! - [`error`] — typed errors (IPC still uses `String`)

pub mod apply;
pub mod classify;
pub mod diag;
pub mod diff;
pub mod error;
pub mod fsutil;
pub mod hash;
pub mod hash_cache;
pub mod ignore_config;
pub mod paths;
pub mod rules;
pub mod snapshot;
pub mod types;
pub mod walk;

pub use apply::{
    apply_plan, preview_apply, recover_incomplete, undo_last, ApplyPreview, ApplyReport,
};
pub use classify::classify;
pub use diff::file_diff;
pub use error::{EngineError, EngineResult};
pub use ignore_config::IgnoreRulesView;
pub use rules::SyncRules;
pub use snapshot::{load_snapshot, save_snapshot, Snapshot};
pub use types::*;
pub use walk::{scan_tree, ScanProgress};
