use crate::engine::hash::{classify_kind, hash_file};
use crate::engine::rules::SyncRules;
use crate::engine::types::{FileEntry, TreeIndex};
use jwalk::WalkDir;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ScanProgress {
    pub files_seen: Arc<AtomicU64>,
    pub files_hashed: Arc<AtomicU64>,
    pub current: Arc<parking_lot::Mutex<String>>,
    pub cancel: Arc<AtomicBool>,
}

impl ScanProgress {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self {
            files_seen: Arc::new(AtomicU64::new(0)),
            files_hashed: Arc::new(AtomicU64::new(0)),
            current: Arc::new(parking_lot::Mutex::new(String::new())),
            cancel,
        }
    }
}

pub fn scan_tree(root: &Path, rules: &SyncRules, progress: &ScanProgress) -> TreeIndex {
    let mut paths: Vec<(String, PathBuf, u64)> = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(false)
        .process_read_dir(|_depth, _path, _state, children| {
            children.retain(|dir_entry| {
                if let Ok(e) = dir_entry {
                    if e.file_type().is_dir() {
                        let rel = e
                            .path()
                            .strip_prefix(root)
                            .unwrap_or_else(|_| e.path().as_path())
                            .to_string_lossy()
                            .replace('\\', "/");
                        // We need rules in closure - use name fast-path only here
                        let name = e.file_name().to_string_lossy();
                        return !matches!(
                            name.as_ref(),
                            "node_modules"
                                | "dist"
                                | "build"
                                | "target"
                                | ".git"
                                | "__pycache__"
                                | ".venv"
                                | "venv"
                                | ".next"
                                | ".turbo"
                                | "coverage"
                        ) && !rules.should_skip_dir(&rel);
                    }
                }
                true
            });
        })
    {
        if progress.cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path();
        let rel = match abs.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if matches!(rules.zone(&rel), crate::engine::types::PathZone::Ignored) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        progress.files_seen.fetch_add(1, Ordering::Relaxed);
        paths.push((rel, abs, size));
    }

    let errors = parking_lot::Mutex::new(Vec::<String>::new());
    let hashed: Vec<Option<(String, FileEntry)>> = paths
        .par_iter()
        .map(|(rel, abs, size)| {
            if progress.cancel.load(Ordering::Relaxed) {
                return None;
            }
            {
                let mut cur = progress.current.lock();
                *cur = rel.clone();
            }
            match hash_file(abs) {
                Ok((hash, real_size)) => {
                    progress.files_hashed.fetch_add(1, Ordering::Relaxed);
                    Some((
                        rel.clone(),
                        FileEntry {
                            rel_path: rel.clone(),
                            size: real_size.max(*size),
                            hash,
                            kind: classify_kind(abs, real_size.max(*size)),
                        },
                    ))
                }
                Err(e) => {
                    errors.lock().push(e);
                    None
                }
            }
        })
        .collect();

    let mut index = TreeIndex::default();
    index.errors = errors.into_inner();
    for item in hashed.into_iter().flatten() {
        index.files.insert(item.0, item.1);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn skips_node_modules() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("app/src")).unwrap();
        fs::create_dir_all(dir.path().join("app/node_modules/pkg")).unwrap();
        fs::write(dir.path().join("app/src/main.ts"), "console.log(1)").unwrap();
        fs::write(dir.path().join("app/node_modules/pkg/index.js"), "x").unwrap();
        let rules = SyncRules::load(dir.path());
        let progress = ScanProgress::new(Arc::new(AtomicBool::new(false)));
        let idx = scan_tree(dir.path(), &rules, &progress);
        assert!(idx.files.contains_key("app/src/main.ts"));
        assert!(!idx.files.keys().any(|k| k.contains("node_modules")));
    }
}
