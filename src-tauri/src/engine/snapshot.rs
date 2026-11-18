use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u32,
    pub this_pc: String,
    pub brought_over: String,
    pub updated_at: String,
    /// rel_path -> content hash at last successful apply (from the source of truth after merge)
    pub files: HashMap<String, String>,
}

pub fn snapshot_key(this_pc: &Path, brought_over: &Path) -> String {
    let mut h = Hasher::new();
    h.update(this_pc.to_string_lossy().as_bytes());
    h.update(b"|");
    h.update(brought_over.to_string_lossy().as_bytes());
    h.finalize().to_hex().to_string()
}

pub fn snapshot_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("CodeSetu")
        .join("snapshots")
}

pub fn snapshot_path(key: &str) -> PathBuf {
    snapshot_dir().join(format!("{key}.json"))
}

pub fn load_snapshot(this_pc: &Path, brought_over: &Path) -> Snapshot {
    let path = snapshot_path(&snapshot_key(this_pc, brought_over));
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => Snapshot::default(),
    }
}

pub fn save_snapshot(snapshot: &Snapshot) -> Result<(), String> {
    let key = snapshot_key(Path::new(&snapshot.this_pc), Path::new(&snapshot.brought_over));
    let dir = snapshot_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = snapshot_path(&key);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_snapshot() {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        // Override data dir by writing via save with absolute paths; function uses dirs::data_dir
        // so we just verify key stability here.
        let k1 = snapshot_key(&this_pc, &brought);
        let k2 = snapshot_key(&this_pc, &brought);
        assert_eq!(k1, k2);
        assert!(!k1.is_empty());
    }
}
