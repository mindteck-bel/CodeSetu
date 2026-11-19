use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub applied: u64,
    /// Intentional Keep mine — destination bytes unchanged; baseline updated.
    #[serde(default)]
    pub kept: u64,
    /// Unresolved Skip, or rows that cannot be applied (env/secret zones).
    pub skipped: u64,
    pub failed: u64,
    pub failures: Vec<String>,
    pub journal_id: Option<String>,
    #[serde(default)]
    pub creates: u64,
    #[serde(default)]
    pub overwrites: u64,
    #[serde(default)]
    pub deletes: u64,
    #[serde(default)]
    pub recovered: bool,
    #[serde(default)]
    pub message: Option<String>,
}
