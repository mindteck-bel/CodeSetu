//! Named project profiles — local folder-pair shortcuts for pilot reuse.
//!
//! Stored under the CodeSetu data directory. No cloud; no accounts.

use crate::engine::paths::codesetu_data_dir;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;
use uuid::Uuid;

fn profiles_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn profiles_path() -> std::path::PathBuf {
    codesetu_data_dir().join("profiles.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProfile {
    pub id: String,
    pub name: String,
    /// Compare mode: `airgap` | `flexible`
    pub mode: String,
    pub this_pc: String,
    pub brought_over: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ProfilesFile {
    version: u32,
    profiles: Vec<ProjectProfile>,
}

fn load_unlocked() -> ProfilesFile {
    let path = profiles_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return ProfilesFile {
            version: 1,
            profiles: vec![],
        };
    };
    serde_json::from_str(&raw).unwrap_or(ProfilesFile {
        version: 1,
        profiles: vec![],
    })
}

fn save_unlocked(file: &ProfilesFile) -> Result<(), String> {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw.as_bytes()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_profiles() -> Vec<ProjectProfile> {
    let _guard = profiles_lock().lock();
    let mut file = load_unlocked();
    file.profiles
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    file.profiles
}

pub fn save_profile(
    name: String,
    mode: String,
    this_pc: String,
    brought_over: String,
    replace_id: Option<String>,
) -> Result<ProjectProfile, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Profile name cannot be empty.".into());
    }
    if name.len() > 80 {
        return Err("Profile name is too long (max 80 characters).".into());
    }
    if this_pc.trim().is_empty() || brought_over.trim().is_empty() {
        return Err("Both folders are required to save a profile.".into());
    }
    let mode = match mode.as_str() {
        "airgap" | "flexible" => mode,
        _ => "airgap".to_string(),
    };
    let _guard = profiles_lock().lock();
    let mut file = load_unlocked();
    let now = chrono::Utc::now().to_rfc3339();
    let profile = if let Some(id) = replace_id {
        if let Some(existing) = file.profiles.iter_mut().find(|p| p.id == id) {
            existing.name = name.clone();
            existing.mode = mode.clone();
            existing.this_pc = this_pc.clone();
            existing.brought_over = brought_over.clone();
            existing.updated_at = now.clone();
            existing.clone()
        } else {
            let p = ProjectProfile {
                id,
                name: name.clone(),
                mode: mode.clone(),
                this_pc: this_pc.clone(),
                brought_over: brought_over.clone(),
                updated_at: now.clone(),
            };
            file.profiles.push(p.clone());
            p
        }
    } else {
        // Upsert by name (case-insensitive) so pilots don't duplicate.
        if let Some(existing) = file
            .profiles
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&name))
        {
            existing.mode = mode.clone();
            existing.this_pc = this_pc.clone();
            existing.brought_over = brought_over.clone();
            existing.updated_at = now.clone();
            existing.clone()
        } else {
            let p = ProjectProfile {
                id: Uuid::new_v4().to_string(),
                name: name.clone(),
                mode: mode.clone(),
                this_pc: this_pc.clone(),
                brought_over: brought_over.clone(),
                updated_at: now.clone(),
            };
            file.profiles.push(p.clone());
            p
        }
    };
    file.version = 1;
    save_unlocked(&file)?;
    Ok(profile)
}

pub fn delete_profile(id: String) -> Result<(), String> {
    let _guard = profiles_lock().lock();
    let mut file = load_unlocked();
    let before = file.profiles.len();
    file.profiles.retain(|p| p.id != id);
    if file.profiles.len() == before {
        return Err("Profile not found.".into());
    }
    save_unlocked(&file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::paths::with_test_data_dir;
    use tempfile::tempdir;

    #[test]
    fn save_list_delete_roundtrip() {
        let dir = tempdir().unwrap();
        with_test_data_dir(dir.path(), || {
            let p = save_profile(
                "Demo".into(),
                "flexible".into(),
                r"C:\dest".into(),
                r"D:\src".into(),
                None,
            )
            .unwrap();
            assert_eq!(list_profiles().len(), 1);
            // Same name upserts
            let p2 = save_profile(
                "demo".into(),
                "airgap".into(),
                r"C:\dest2".into(),
                r"D:\src2".into(),
                None,
            )
            .unwrap();
            assert_eq!(p.id, p2.id);
            assert_eq!(list_profiles().len(), 1);
            assert_eq!(list_profiles()[0].mode, "airgap");
            delete_profile(p.id).unwrap();
            assert!(list_profiles().is_empty());
        });
    }
}
