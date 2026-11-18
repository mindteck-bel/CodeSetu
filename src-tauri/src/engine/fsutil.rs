//! Cross-platform durable file helpers (replace, fsync).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// Copy `src` → temp beside `dest`, fsync via a **write** handle, then replace
/// `dest` (Windows-safe `MoveFileExW`).
pub fn replace_file(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("source missing: {}", src.display()));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = temp_beside(dest);
    // Ensure no stale temp from a prior crash.
    let _ = remove_file_resilient(&tmp);

    // Copy + sync on the same write handle so Windows FlushFileBuffers works.
    // (A read-only reopen + sync_all fails with Access Denied / os error 5.)
    {
        let mut inp = File::open(src).map_err(|e| format!("open source: {e}"))?;
        let mut out = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|e| format!("create temp: {e}"))?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = inp.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
        out.sync_all()
            .map_err(|e| format!("fsync temp {}: {e}", tmp.display()))?;
        drop(out);
        drop(inp);
    }

    durable_rename(&tmp, dest)?;
    Ok(())
}

/// Write bytes via temp + fsync (writer handle) + replace.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = temp_beside(path);
    let _ = remove_file_resilient(&tmp);
    {
        let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all()
            .map_err(|e| format!("fsync temp {}: {e}", tmp.display()))?;
        // Explicit close before rename (important on Windows).
        drop(f);
    }
    durable_rename(&tmp, path)?;
    Ok(())
}

/// Durably flush file contents. On Windows the handle must be writable —
/// `File::open` (read-only) + `sync_all` fails with Access Denied.
pub fn fsync_path(path: &Path) -> Result<(), String> {
    {
        let f = fs::OpenOptions::new()
            .write(true)
            .read(true)
            .open(path)
            .map_err(|e| format!("fsync open {}: {e}", path.display()))?;
        f.sync_all()
            .map_err(|e| format!("fsync {}: {e}", path.display()))?;
        drop(f);
    }
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

pub fn clear_readonly(path: &Path) {
    // Readonly-bit clearing is only needed on Windows for overwrite/delete.
    // On Unix, `set_readonly(false)` would make the file world-writable — skip.
    #[cfg(windows)]
    {
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                {
                    perms.set_readonly(false);
                }
                let _ = fs::set_permissions(path, perms);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
}

/// Remove a file, clearing readonly and retrying on sharing/access errors.
pub fn remove_file_resilient(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    clear_readonly(path);
    #[cfg(windows)]
    {
        win::remove_resilient(path)
    }
    #[cfg(not(windows))]
    {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

/// Rename with replace semantics.
///
/// Windows: `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` (do **not** delete
/// the destination first — that races with AV and fails with Access Denied).
/// Unix: `rename(2)` replaces atomically.
pub fn durable_rename(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::replace_rename(from, to)
    }
    #[cfg(not(windows))]
    {
        fs::rename(from, to).map_err(|e| e.to_string())
    }
}

/// Overwrite `to` by streaming from `from` (last-resort Windows fallback).
pub fn copy_over_inplace(from: &Path, to: &Path) -> Result<(), String> {
    clear_readonly(to);
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut inp = File::open(from).map_err(|e| e.to_string())?;
    let mut out = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(to)
        .map_err(|e| e.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = inp.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    out.sync_all().map_err(|e| e.to_string())?;
    drop(out);
    drop(inp);
    let _ = remove_file_resilient(from);
    Ok(())
}

#[cfg(windows)]
mod win {
    use super::{clear_readonly, copy_over_inplace};
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_SHARING_VIOLATION: u32 = 32;
    const ERROR_LOCK_VIOLATION: u32 = 33;

    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;
        fn GetLastError() -> u32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    fn retryable(err: u32) -> bool {
        matches!(
            err,
            ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION
        )
    }

    pub fn replace_rename(from: &Path, to: &Path) -> Result<(), String> {
        clear_readonly(to);
        clear_readonly(from);
        let from_w = wide(from);
        let to_w = wide(to);
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;

        for attempt in 0..10 {
            let ok = unsafe { MoveFileExW(from_w.as_ptr(), to_w.as_ptr(), flags) };
            if ok != 0 {
                return Ok(());
            }
            let err = unsafe { GetLastError() };
            if retryable(err) && attempt < 9 {
                thread::sleep(Duration::from_millis(15 * (1 << attempt.min(5))));
                clear_readonly(to);
                clear_readonly(from);
                continue;
            }
            // Last resort: stream overwrite in place, then delete temp.
            match copy_over_inplace(from, to) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    return Err(format!(
                        "MoveFileExW failed (winerror {err}) replacing {} <- {}: {e}",
                        to.display(),
                        from.display()
                    ));
                }
            }
        }
        Err(format!(
            "MoveFileExW failed after retries: {} <- {}",
            to.display(),
            from.display()
        ))
    }

    pub fn remove_resilient(path: &Path) -> Result<(), String> {
        clear_readonly(path);
        for attempt in 0..10 {
            match fs::remove_file(path) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let code = e.raw_os_error().unwrap_or(0) as u32;
                    if retryable(code) && attempt < 9 {
                        thread::sleep(Duration::from_millis(15 * (1 << attempt.min(5))));
                        clear_readonly(path);
                        continue;
                    }
                    // Treat already-gone as success.
                    if e.kind() == std::io::ErrorKind::NotFound {
                        return Ok(());
                    }
                    return Err(format!("cannot remove {}: {e}", path.display()));
                }
            }
        }
        Err(format!("cannot remove {} after retries", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fsync_path_works_after_write() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("x.txt");
        fs::write(&p, b"data").unwrap();
        // Must not Access Denied on Windows (needs write-capable open).
        fsync_path(&p).unwrap();
    }

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
    fn overwrite_existing_twice() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&src, b"v1").unwrap();
        fs::write(&dest, b"old").unwrap();
        replace_file(&src, &dest).unwrap();
        fs::write(&src, b"v2").unwrap();
        replace_file(&src, &dest).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "v2");
    }

    #[test]
    fn write_atomic_roundtrip() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("a.json");
        write_atomic(&p, br#"{"ok":true}"#).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), r#"{"ok":true}"#);
        write_atomic(&p, br#"{"ok":false}"#).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), r#"{"ok":false}"#);
    }

    #[test]
    fn remove_file_resilient_deletes() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("gone.txt");
        fs::write(&p, b"x").unwrap();
        remove_file_resilient(&p).unwrap();
        assert!(!p.exists());
    }
}
