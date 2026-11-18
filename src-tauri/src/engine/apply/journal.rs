//! Apply journal types and durable persistence helpers.

use crate::engine::fsutil::write_atomic;
use crate::engine::paths::codesetu_data_dir;
use serde::{Deserialize, Serialize};
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

pub fn journal_dir() -> PathBuf {
    codesetu_data_dir().join("journals")
}

pub fn backup_dir(session_id: &str) -> PathBuf {
    codesetu_data_dir().join("backups").join(session_id)
}

pub fn inprogress_path(session_id: &str) -> PathBuf {
    journal_dir().join(format!("{session_id}.inprogress.json"))
}

pub fn committed_path(session_id: &str) -> PathBuf {
    journal_dir().join(format!("{session_id}.json"))
}

pub fn persist_journal(journal: &Journal, path: &Path) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?;
    write_atomic(path, raw.as_bytes())
}
