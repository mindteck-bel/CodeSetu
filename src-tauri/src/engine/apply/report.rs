use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub applied: u64,
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
