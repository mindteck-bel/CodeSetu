//! Dry-run counts for the apply confirm modal.

use crate::engine::types::{ChangeClass, Decision, FileRow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPreview {
    pub creates: u64,
    pub overwrites: u64,
    pub deletes: u64,
    pub moves: u64,
    pub skips: u64,
    pub destructive: bool,
    pub rows_to_apply: u64,
    /// True when there is no prior baseline for this folder pair.
    #[serde(default)]
    pub first_sync: bool,
    /// When set, preview/apply is limited to this many selected row ids.
    #[serde(default)]
    pub selected_count: u64,
}

/// When `only_ids` is `Some`, only those row ids are counted; others are out of scope.
pub fn preview_apply(
    rows: &[FileRow],
    first_sync: bool,
    only_ids: Option<&[String]>,
) -> ApplyPreview {
    let filter: Option<HashSet<&str>> =
        only_ids.map(|ids| ids.iter().map(|s| s.as_str()).collect());
    let mut preview = ApplyPreview {
        first_sync,
        selected_count: filter.as_ref().map(|f| f.len() as u64).unwrap_or(0),
        ..Default::default()
    };
    for row in rows {
        if let Some(ref allowed) = filter {
            if !allowed.contains(row.id.as_str()) {
                continue;
            }
        }
        if matches!(
            row.class,
            ChangeClass::IntranetOnly | ChangeClass::InternetOnly | ChangeClass::ParkedSecret
        ) {
            preview.skips += 1;
            continue;
        }
        match row.decision {
            Decision::Keep | Decision::Skip => {
                preview.skips += 1;
            }
            Decision::BringIn => {
                preview.rows_to_apply += 1;
                if matches!(row.class, ChangeClass::DeletedBrought) {
                    preview.deletes += 1;
                    preview.destructive = true;
                } else if row.moved_from.is_some() {
                    preview.moves += 1;
                    preview.overwrites += 1;
                    preview.destructive = true;
                } else if row.this_hash.is_some() {
                    preview.overwrites += 1;
                    preview.destructive = true;
                } else {
                    preview.creates += 1;
                }
            }
        }
    }
    preview
}
