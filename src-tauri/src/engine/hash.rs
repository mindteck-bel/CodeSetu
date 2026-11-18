use crate::engine::types::FileKind;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub const LARGE_TEXT_BYTES: u64 = 2 * 1024 * 1024;

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "pdf", "zip", "7z", "rar", "gz", "tar",
    "exe", "dll", "so", "dylib", "bin", "wasm", "woff", "woff2", "ttf", "otf", "eot", "mp3", "mp4",
    "avi", "mov", "mkv", "class", "jar", "war", "pdb", "obj", "o", "a", "lib", "pyc", "pyo",
];

pub fn classify_kind(path: &Path, size: u64) -> FileKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if BINARY_EXTENSIONS.iter().any(|b| *b == ext) {
        return FileKind::Binary;
    }
    if size > LARGE_TEXT_BYTES {
        return FileKind::Large;
    }
    FileKind::Text
}

pub fn hash_file(path: &Path) -> Result<(String, u64), String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta = file
        .metadata()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let size = meta.len();
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok((hasher.finalize().to_hex().to_string(), size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hashes_are_stable() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("a.txt");
        fs::write(&p, b"hello codesetu").unwrap();
        let (h1, s1) = hash_file(&p).unwrap();
        let (h2, s2) = hash_file(&p).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(s1, s2);
        assert_eq!(s1, 14);
    }
}
