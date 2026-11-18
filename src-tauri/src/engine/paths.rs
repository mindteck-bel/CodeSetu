//! Path confinement helpers — reject traversal and keep joins inside roots.

use parking_lot::Mutex;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

/// Normalize a relative project path: reject absolute, `..`, and empty.
pub fn normalize_rel_path(rel: &str) -> Result<String, String> {
    // Treat Windows separators as separators even on Unix hosts.
    let trimmed = rel.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err("Empty relative path.".into());
    }
    let path = Path::new(&trimmed);
    if path.is_absolute() {
        return Err(format!("Absolute paths are not allowed: {trimmed}"));
    }
    let mut parts: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s.contains('\0') {
                    return Err(format!("Invalid path component in: {trimmed}"));
                }
                parts.push(s.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!("Path traversal rejected: {trimmed}"));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Absolute paths are not allowed: {trimmed}"));
            }
        }
    }
    if parts.is_empty() {
        return Err(format!("Empty relative path after normalize: {trimmed}"));
    }
    Ok(parts.join("/"))
}

/// Join `rel` under `root` and ensure the result stays inside `root`.
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let normalized = normalize_rel_path(rel)?;
    let joined = root.join(&normalized);
    let root_norm = normalize_for_compare(root);
    let joined_norm = normalize_for_compare(&joined);
    if !joined_norm.starts_with(&root_norm) {
        return Err(format!("Path escapes project root: {rel}"));
    }
    Ok(joined)
}

fn normalize_for_compare(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn data_dir_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Optional data-dir override for tests (`CODESETU_DATA_DIR`).
pub fn codesetu_data_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("CODESETU_DATA_DIR") {
        if !override_dir.is_empty() {
            return PathBuf::from(override_dir);
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("CodeSetu")
}

/// Run a closure with an exclusive `CODESETU_DATA_DIR` (tests only).
pub fn with_test_data_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    let _guard = data_dir_lock().lock();
    std::env::set_var("CODESETU_DATA_DIR", dir);
    let out = f();
    std::env::remove_var("CODESETU_DATA_DIR");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_traversal() {
        assert!(normalize_rel_path("../etc/passwd").is_err());
        assert!(normalize_rel_path("a/../../b").is_err());
        assert!(normalize_rel_path("/abs").is_err());
        assert!(normalize_rel_path(r"..\..\windows\system32").is_err());
        assert!(normalize_rel_path(r"foo\..\..\bar").is_err());
    }

    #[test]
    fn accepts_nested() {
        assert_eq!(
            normalize_rel_path(r"src\app\main.rs").unwrap(),
            "src/app/main.rs"
        );
        assert_eq!(normalize_rel_path("./foo/bar").unwrap(), "foo/bar");
    }

    #[test]
    fn safe_join_stays_in_root() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let ok = safe_join(root, "a/b.txt").unwrap();
        assert!(ok.starts_with(root));
        assert!(safe_join(root, "../escape.txt").is_err());
    }
}
