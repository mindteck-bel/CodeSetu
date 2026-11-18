use crate::engine::hash::{classify_kind, LARGE_TEXT_BYTES};
use crate::engine::paths::safe_join;
use crate::engine::types::{DiffHunk, FileDiff, FileKind};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

const MAX_DIFF_CHARS: usize = 400_000;

pub fn file_diff(this_pc: &Path, brought_over: &Path, rel: &str) -> FileDiff {
    let this_path = match safe_join(this_pc, rel) {
        Ok(p) => p,
        Err(e) => {
            return FileDiff {
                rel_path: rel.to_string(),
                kind: FileKind::Binary,
                this_exists: false,
                brought_exists: false,
                hunks: vec![],
                truncated: false,
                error: Some(e),
            };
        }
    };
    let brought_path = match safe_join(brought_over, rel) {
        Ok(p) => p,
        Err(e) => {
            return FileDiff {
                rel_path: rel.to_string(),
                kind: FileKind::Binary,
                this_exists: false,
                brought_exists: false,
                hunks: vec![],
                truncated: false,
                error: Some(e),
            };
        }
    };
    let this_exists = this_path.exists();
    let brought_exists = brought_path.exists();

    let size = this_path
        .metadata()
        .map(|m| m.len())
        .or_else(|_| brought_path.metadata().map(|m| m.len()))
        .unwrap_or(0);
    let kind_path = if this_path.exists() {
        this_path.as_path()
    } else {
        brought_path.as_path()
    };
    let kind = classify_kind(kind_path, size);

    if matches!(kind, FileKind::Binary | FileKind::Large) || size > LARGE_TEXT_BYTES {
        return FileDiff {
            rel_path: rel.to_string(),
            kind,
            this_exists,
            brought_exists,
            hunks: vec![],
            truncated: false,
            error: None,
        };
    }

    let this_text = if this_exists {
        match fs::read_to_string(&this_path) {
            Ok(s) => s,
            Err(e) => {
                return FileDiff {
                    rel_path: rel.to_string(),
                    kind: FileKind::Binary,
                    this_exists,
                    brought_exists,
                    hunks: vec![],
                    truncated: false,
                    error: Some(e.to_string()),
                };
            }
        }
    } else {
        String::new()
    };

    let brought_text = if brought_exists {
        match fs::read_to_string(&brought_path) {
            Ok(s) => s,
            Err(e) => {
                return FileDiff {
                    rel_path: rel.to_string(),
                    kind: FileKind::Binary,
                    this_exists,
                    brought_exists,
                    hunks: vec![],
                    truncated: false,
                    error: Some(e.to_string()),
                };
            }
        }
    } else {
        String::new()
    };

    let truncated = this_text.len() + brought_text.len() > MAX_DIFF_CHARS;
    let left = if truncated {
        truncate(&this_text)
    } else {
        this_text.as_str()
    };
    let right = if truncated {
        truncate(&brought_text)
    } else {
        brought_text.as_str()
    };

    let diff = TextDiff::from_lines(left, right);
    let mut hunks = Vec::new();
    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Delete => "delete",
            ChangeTag::Insert => "insert",
        };
        let value = change.to_string();
        hunks.push(DiffHunk {
            tag: tag.into(),
            this_text: if tag != "insert" {
                value.clone()
            } else {
                String::new()
            },
            brought_text: if tag != "delete" {
                value
            } else {
                String::new()
            },
        });
        if hunks.len() > 4000 {
            return FileDiff {
                rel_path: rel.to_string(),
                kind: FileKind::Text,
                this_exists,
                brought_exists,
                hunks,
                truncated: true,
                error: None,
            };
        }
    }

    FileDiff {
        rel_path: rel.to_string(),
        kind: FileKind::Text,
        this_exists,
        brought_exists,
        hunks,
        truncated,
        error: None,
    }
}

fn truncate(s: &str) -> &str {
    let mut end = MAX_DIFF_CHARS / 2;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end.min(s.len())]
}
