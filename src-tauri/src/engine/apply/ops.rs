//! Per-file backup / durable copy helpers used by apply.

use crate::engine::fsutil::{fsync_path, remove_file_resilient, replace_file};
use crate::engine::hash::hash_file;
use std::fs;
use std::path::Path;

pub fn backup_and_remove(dest: &Path, backups: &Path, rel: &str) -> Result<Option<String>, String> {
    if !dest.exists() {
        return Ok(None);
    }
    let backup = backups.join(rel);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(dest, &backup).map_err(|e| e.to_string())?;
    fsync_path(&backup).ok();
    remove_file_resilient(dest)?;
    Ok(Some(backup.display().to_string()))
}

pub fn copy_atomic(
    src: &Path,
    dest: &Path,
    backups: &Path,
    rel: &str,
) -> Result<(Option<String>, String), String> {
    if !src.exists() {
        return Err(format!("source missing: {}", src.display()));
    }
    let mut backup = None;
    if dest.exists() {
        let b = backups.join(rel);
        if let Some(parent) = b.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(dest, &b).map_err(|e| e.to_string())?;
        fsync_path(&b).ok();
        backup = Some(b.display().to_string());
        // Keep dest in place until replace — Windows MoveFileEx replaces without
        // a prior delete (delete-first races AV and yields Access Denied).
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Prefer shared helper (temp + fsync + durable replace).
    replace_file(src, dest).inspect_err(|_e| {
        // Best-effort temp cleanup if replace_file left one behind.
        let tmp = dest.with_file_name(format!(
            "{}.codesetu.tmp",
            dest.file_name().and_then(|s| s.to_str()).unwrap_or("file")
        ));
        let _ = remove_file_resilient(&tmp);
    })?;
    let (hash, _) = hash_file(dest)?;
    Ok((backup, hash))
}
