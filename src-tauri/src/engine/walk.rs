use crate::engine::hash::{classify_kind, hash_file};
use crate::engine::hash_cache;
use crate::engine::rules::SyncRules;
use crate::engine::types::{FileEntry, TreeIndex};
use jwalk::WalkDir;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ScanProgress {
    pub files_seen: Arc<AtomicU64>,
    pub files_hashed: Arc<AtomicU64>,
    pub files_cache_hits: Arc<AtomicU64>,
    pub current: Arc<Mutex<String>>,
    pub cancel: Arc<AtomicBool>,
}

impl ScanProgress {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self {
            files_seen: Arc::new(AtomicU64::new(0)),
            files_hashed: Arc::new(AtomicU64::new(0)),
            files_cache_hits: Arc::new(AtomicU64::new(0)),
            current: Arc::new(Mutex::new(String::new())),
            cancel,
        }
    }
}

fn should_prune_dir_name(name: &str) -> bool {
    matches!(
        name,
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
            | "bin"
            | "obj"
            | ".cache"
            | "tmp"
            | ".svn"
            | ".hg"
            | ".idea"
            | ".vscode"
    )
}

pub fn scan_tree(root: &Path, rules: &SyncRules, progress: &ScanProgress) -> TreeIndex {
    let mut paths: Vec<(String, PathBuf, u64)> = Vec::new();
    let cache = Arc::new(Mutex::new(hash_cache::load_cache(root)));
    let root_buf = root.to_path_buf();

    for entry in WalkDir::new(&root_buf)
        .follow_links(false)
        .skip_hidden(false)
        .process_read_dir(|_depth, path, _state, children| {
            children.retain(|dir_entry| {
                if let Ok(e) = dir_entry {
                    if e.file_type().is_dir() {
                        let name = e.file_name().to_string_lossy();
                        if should_prune_dir_name(name.as_ref()) {
                            return false;
                        }
                        // Also prune by relative path name segments for nested ignores
                        let rel = e
                            .path()
                            .strip_prefix(path)
                            .ok()
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default();
                        let _ = rel;
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
        if entry.file_type().is_dir() {
            // Double-check custom ignore dirs that weren't pruned by name.
            let abs = entry.path();
            if let Ok(rel) = abs.strip_prefix(root) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if rules.should_skip_dir(&rel) {
                    // jwalk has already descended; files under it are still filtered below.
                }
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path();
        let rel = match abs.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        // Skip anything under an ignored directory prefix
        if rel.split('/').any(should_prune_dir_name) {
            continue;
        }
        if rules.should_skip_dir(
            Path::new(&rel)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default()
                .as_str(),
        ) {
            continue;
        }
        if matches!(rules.zone(&rel), crate::engine::types::PathZone::Ignored) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        progress.files_seen.fetch_add(1, Ordering::Relaxed);
        paths.push((rel, abs, size));
    }

    let errors = Mutex::new(Vec::<String>::new());
    let cache_for_workers = cache.clone();

    let hashed: Vec<Option<(String, FileEntry)>> = paths
        .par_iter()
        .map(|(rel, abs, size)| {
            if progress.cancel.load(Ordering::Relaxed) {
                return None;
            }
            let n = progress.files_hashed.load(Ordering::Relaxed)
                + progress.files_cache_hits.load(Ordering::Relaxed);
            if n % 32 == 0 {
                let mut cur = progress.current.lock();
                *cur = rel.clone();
            }

            {
                let cache_guard = cache_for_workers.lock();
                if let Some(hash) = hash_cache::lookup(&cache_guard, rel, abs) {
                    progress.files_cache_hits.fetch_add(1, Ordering::Relaxed);
                    progress.files_hashed.fetch_add(1, Ordering::Relaxed);
                    return Some((
                        rel.clone(),
                        FileEntry {
                            rel_path: rel.clone(),
                            size: *size,
                            hash,
                            kind: classify_kind(abs, *size),
                        },
                    ));
                }
            }

            match hash_file(abs) {
                Ok((hash, real_size)) => {
                    progress.files_hashed.fetch_add(1, Ordering::Relaxed);
                    {
                        let mut cache_guard = cache_for_workers.lock();
                        hash_cache::store(&mut cache_guard, rel, abs, hash.clone());
                    }
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

    {
        let guard = cache.lock();
        let _ = hash_cache::save_cache(root, &guard);
    }

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
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            fs::create_dir_all(dir.path().join("app/src")).unwrap();
            fs::create_dir_all(dir.path().join("app/node_modules/pkg")).unwrap();
            fs::write(dir.path().join("app/src/main.ts"), "console.log(1)").unwrap();
            fs::write(dir.path().join("app/node_modules/pkg/index.js"), "x").unwrap();
            let rules = SyncRules::load(dir.path());
            let progress = ScanProgress::new(Arc::new(AtomicBool::new(false)));
            let idx = scan_tree(dir.path(), &rules, &progress);
            assert!(idx.files.contains_key("app/src/main.ts"));
            assert!(!idx.files.keys().any(|k| k.contains("node_modules")));
        });
    }

    #[test]
    fn does_not_follow_symlinks() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let root = dir.path().join("proj");
            let outside = dir.path().join("outside");
            fs::create_dir_all(root.join("src")).unwrap();
            fs::create_dir_all(&outside).unwrap();
            fs::write(root.join("src/ok.txt"), b"inside").unwrap();
            fs::write(outside.join("secret.txt"), b"outside").unwrap();
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&outside, root.join("link-out")).unwrap();
            }
            #[cfg(windows)]
            {
                // Junction/symlink may require privileges; skip soft-fail if creation fails.
                let _ = std::os::windows::fs::symlink_dir(&outside, root.join("link-out"));
            }
            let rules = SyncRules::load(&root);
            let progress = ScanProgress::new(Arc::new(AtomicBool::new(false)));
            let idx = scan_tree(&root, &rules, &progress);
            assert!(idx.files.contains_key("src/ok.txt"));
            assert!(
                !idx.files.keys().any(|k| k.contains("secret.txt")),
                "symlink targets must not be scanned: {:?}",
                idx.files.keys().collect::<Vec<_>>()
            );
        });
    }
}
