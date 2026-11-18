use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Text,
    Binary,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PathZone {
    Shared,
    IntranetOnly,
    InternetOnly,
    ParkedSecret,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeClass {
    Identical,
    BringIn,
    Keep,
    NeedsYou,
    AddedBrought,
    DeletedBrought,
    Moved,
    IntranetOnly,
    InternetOnly,
    ParkedSecret,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    BringIn,
    Keep,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub rel_path: String,
    pub size: u64,
    pub hash: String,
    pub kind: FileKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRow {
    pub id: String,
    pub rel_path: String,
    pub group: String,
    pub class: ChangeClass,
    pub decision: Decision,
    pub zone: PathZone,
    pub kind: FileKind,
    pub this_size: Option<u64>,
    pub brought_size: Option<u64>,
    pub this_hash: Option<String>,
    pub brought_hash: Option<String>,
    pub moved_from: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub total_this: u64,
    pub total_brought: u64,
    pub hashed: u64,
    pub identical: u64,
    pub bring_in: u64,
    pub keep: u64,
    pub needs_you: u64,
    pub intranet_only: u64,
    pub internet_only: u64,
    pub parked_secret: u64,
    pub deleted_brought: u64,
    pub moved: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    /// True when no prior baseline exists for this folder pair.
    #[serde(default)]
    pub first_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub session_id: String,
    pub this_pc: String,
    pub brought_over: String,
    pub summary: ScanSummary,
    pub rows: Vec<FileRow>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub session_id: String,
    pub phase: String,
    pub files_seen: u64,
    pub files_hashed: u64,
    pub current_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub tag: String,
    pub this_text: String,
    pub brought_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub rel_path: String,
    pub kind: FileKind,
    pub this_exists: bool,
    pub brought_exists: bool,
    pub hunks: Vec<DiffHunk>,
    pub truncated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct TreeIndex {
    pub files: HashMap<String, FileEntry>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SessionPaths {
    pub this_pc: PathBuf,
    pub brought_over: PathBuf,
}
