use std::path::PathBuf;

#[tauri::command]
pub fn validate_folder(path: String) -> Result<bool, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("Folder does not exist. Choose an existing directory.".into());
    }
    if !p.is_dir() {
        return Err("Path is not a folder. Choose a directory, not a file.".into());
    }
    match std::fs::read_dir(&p) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Cannot read folder (check permissions): {e}")),
    }
}
