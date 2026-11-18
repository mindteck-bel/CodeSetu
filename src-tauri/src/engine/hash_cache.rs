//! Metadata-based content hash cache for incremental scans.

use crate::engine::fsutil::write_atomic;
use crate::engine::paths::codesetu_data_dir;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    pub size: u64,
    pub mtime_secs: u64,
    pub mtime_nanos: u32,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HashCache {
    pub version: u32,
    pub root: String,
    pub entries: HashMap<String, CacheEntry>,
}

fn cache_key(root: &Path) -> String {
    let mut h = Hasher::new();
    h.update(root.to_string_lossy().as_bytes());
    h.finalize().to_hex().to_string()
}

fn cache_path(root: &Path) -> PathBuf {
    codesetu_data_dir()
        .join("hash-cache")
        .join(format!("{}.json", cache_key(root)))
}

pub fn load_cache(root: &Path) -> HashCache {
    let path = cache_path(root);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| HashCache {
            version: 1,
            root: root.display().to_string(),
            entries: HashMap::new(),
        }),
        Err(_) => HashCache {
            version: 1,
            root: root.display().to_string(),
            entries: HashMap::new(),
        },
    }
}

pub fn save_cache(root: &Path, cache: &HashCache) -> Result<(), String> {
    let path = cache_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string(cache).map_err(|e| e.to_string())?;
    write_atomic(&path, raw.as_bytes())
}

pub fn meta_of(path: &Path) -> Option<(u64, u64, u32)> {
    let meta = fs::metadata(path).ok()?;
    let size = meta.len();
    let modified = meta.modified().ok()?;
    let dur = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Some((size, dur.as_secs(), dur.subsec_nanos()))
}

pub fn lookup(cache: &HashCache, rel: &str, path: &Path) -> Option<String> {
    let entry = cache.entries.get(rel)?;
    let (size, secs, nanos) = meta_of(path)?;
    if entry.size == size && entry.mtime_secs == secs && entry.mtime_nanos == nanos {
        Some(entry.hash.clone())
    } else {
        None
    }
}

pub fn store(cache: &mut HashCache, rel: &str, path: &Path, hash: String) {
    if let Some((size, secs, nanos)) = meta_of(path) {
        cache.entries.insert(
            rel.to_string(),
            CacheEntry {
                size,
                mtime_secs: secs,
                mtime_nanos: nanos,
                hash,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::hash::hash_file;
    use tempfile::tempdir;

    #[test]
    fn cache_hit_and_invalidate() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(dir.path(), || {
            let root = dir.path().join("tree");
            fs::create_dir_all(&root).unwrap();
            let file = root.join("a.txt");
            fs::write(&file, b"one").unwrap();
            // Ensure metadata is stable after write
            let (h1, _) = hash_file(&file).unwrap();
            let mut cache = HashCache {
                version: 1,
                root: root.display().to_string(),
                entries: Default::default(),
            };
            store(&mut cache, "a.txt", &file, h1.clone());
            assert_eq!(
                lookup(&cache, "a.txt", &file).as_deref(),
                Some(h1.as_str()),
                "in-memory lookup should hit"
            );
            save_cache(&root, &cache).unwrap();

            let cache2 = load_cache(&root);
            assert_eq!(
                lookup(&cache2, "a.txt", &file).as_deref(),
                Some(h1.as_str()),
                "persisted lookup should hit"
            );

            std::thread::sleep(std::time::Duration::from_millis(50));
            fs::write(&file, b"two").unwrap();
            assert!(lookup(&cache2, "a.txt", &file).is_none());
        });
    }
}
