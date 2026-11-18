use crate::engine::fsutil::write_atomic;
use crate::engine::paths::codesetu_data_dir;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u32,
    pub this_pc: String,
    pub brought_over: String,
    pub updated_at: String,
    /// Generation bumped on each successful apply commit (for undo restore).
    #[serde(default)]
    pub generation: u64,
    /// rel_path -> content hash at last successful apply / identical commit
    pub files: HashMap<String, String>,
    /// Paths the user chose to keep locally despite absence on brought-over.
    #[serde(default)]
    pub keep_despite_absent: HashSet<String>,
    /// Last known brought-over file count (for incomplete-copy heuristics).
    #[serde(default)]
    pub last_brought_file_count: u64,
}

pub fn snapshot_key(this_pc: &Path, brought_over: &Path) -> String {
    let mut h = Hasher::new();
    h.update(canonicalize_key_bytes(this_pc).as_bytes());
    h.update(b"|");
    h.update(canonicalize_key_bytes(brought_over).as_bytes());
    h.finalize().to_hex().to_string()
}

fn canonicalize_key_bytes(path: &Path) -> String {
    let display = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_string();
    display.replace('\\', "/").to_ascii_lowercase()
}

pub fn snapshot_dir() -> PathBuf {
    codesetu_data_dir().join("snapshots")
}

pub fn snapshot_path(key: &str) -> PathBuf {
    snapshot_dir().join(format!("{key}.json"))
}

pub fn snapshot_path_for(this_pc: &Path, brought_over: &Path) -> PathBuf {
    snapshot_path(&snapshot_key(this_pc, brought_over))
}

pub fn load_snapshot(this_pc: &Path, brought_over: &Path) -> Snapshot {
    let path = snapshot_path_for(this_pc, brought_over);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => Snapshot::default(),
    }
}

pub fn save_snapshot(snapshot: &Snapshot) -> Result<(), String> {
    let key = snapshot_key(
        Path::new(&snapshot.this_pc),
        Path::new(&snapshot.brought_over),
    );
    let dir = snapshot_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = snapshot_path(&key);
    let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    write_atomic(&path, json.as_bytes())
}

/// Persist a snapshot blob to an arbitrary path (used for pre-apply backup in journal).
pub fn save_snapshot_to(path: &Path, snapshot: &Snapshot) -> Result<(), String> {
    let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    write_atomic(path, json.as_bytes())
}

pub fn load_snapshot_from(path: &Path) -> Result<Snapshot, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_snapshot() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(dir.path(), || {
            let this_pc = dir.path().join("intra");
            let brought = dir.path().join("inet");
            fs::create_dir_all(&this_pc).unwrap();
            fs::create_dir_all(&brought).unwrap();
            let k1 = snapshot_key(&this_pc, &brought);
            let k2 = snapshot_key(&this_pc, &brought);
            assert_eq!(k1, k2);
            assert!(!k1.is_empty());

            let mut snap = Snapshot::default();
            snap.this_pc = this_pc.display().to_string();
            snap.brought_over = brought.display().to_string();
            snap.files.insert("a.txt".into(), "abc".into());
            snap.keep_despite_absent.insert("kept.txt".into());
            snap.generation = 3;
            save_snapshot(&snap).unwrap();
            let loaded = load_snapshot(&this_pc, &brought);
            assert_eq!(loaded.files.get("a.txt").map(String::as_str), Some("abc"));
            assert!(loaded.keep_despite_absent.contains("kept.txt"));
            assert_eq!(loaded.generation, 3);
        });
    }

    #[test]
    fn trailing_slash_same_key() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("p");
        fs::create_dir_all(&a).unwrap();
        let with = PathBuf::from(format!("{}", a.display()) + "/");
        // canonicalize may fail for trailing slash on some OS; compare normalize helper
        let k1 = canonicalize_key_bytes(&a);
        let k2 = canonicalize_key_bytes(&with);
        assert_eq!(k1, k2);
    }
}
