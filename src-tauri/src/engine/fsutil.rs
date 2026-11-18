//! Cross-platform durable file helpers (replace, fsync).

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Copy `src` → temp beside `dest`, fsync, then replace `dest` (Windows-safe).
pub fn replace_file(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("source missing: {}", src.display()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = temp_beside(dest);
    fs::copy(src, &tmp).map_err(|e| format!("temp copy failed: {e}"))?;
    fsync_path(&tmp)?;
    durable_rename(&tmp, dest)?;
    Ok(())
}

/// Write bytes via temp + fsync + replace.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = temp_beside(path);
    {
        let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    durable_rename(&tmp, path)?;
    Ok(())
}

pub fn fsync_path(path: &Path) -> Result<(), String> {
    let f = File::open(path).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    // Best-effort directory fsync on Unix.
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

fn temp_beside(dest: &Path) -> std::path::PathBuf {
    let stem = dest.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    dest.with_file_name(format!("{stem}.codesetu.tmp"))
}

/// Rename with replace semantics. On Windows, `rename` fails if `dest` exists,
/// so remove the destination first (caller must have backed it up).
pub fn durable_rename(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        if to.exists() {
            fs::remove_file(to).map_err(|e| {
                format!(
                    "cannot replace existing file on Windows ({}): {e}",
                    to.display()
                )
            })?;
        }
        match fs::rename(from, to) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Fallback: copy + remove temp
                match fs::copy(from, to) {
                    Ok(_) => {
                        let _ = fs::remove_file(from);
                        Ok(())
                    }
                    Err(e2) => Err(format!("replace failed: {e}; fallback copy: {e2}")),
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(from, to).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn overwrite_existing() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&src, b"new").unwrap();
        fs::write(&dest, b"old").unwrap();
        replace_file(&src, &dest).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new");
        assert!(!dir.path().join("dest.txt.codesetu.tmp").exists());
    }

    #[test]
    fn write_atomic_roundtrip() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_atomic(&p, br#"{"ok":true}"#).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), r#"{"ok":true}"#);
    }
}
