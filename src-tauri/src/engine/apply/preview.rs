//! Dry-run counts for the apply confirm modal.

use crate::engine::types::{ChangeClass, Decision, FileRow};
use serde::{Deserialize, Serialize};

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
}

pub fn preview_apply(rows: &[FileRow], first_sync: bool) -> ApplyPreview {
    let mut preview = ApplyPreview {
        first_sync,
        ..Default::default()
    };
    for row in rows {
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
