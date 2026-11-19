//! Phase 2 pilot commands: diagnostics export, sync history, project profiles.

use crate::engine::{
    delete_profile, export_diagnostic_report, list_profiles, list_sync_history, save_profile,
    HistoryEntry, ProjectProfile,
};
use std::path::PathBuf;

#[tauri::command]
pub fn export_diagnostics(dest_path: String) -> Result<String, String> {
    let path = PathBuf::from(dest_path.trim());
    let written = export_diagnostic_report(&path)?;
    Ok(written.display().to_string())
}

#[tauri::command]
pub fn get_sync_history(limit: Option<u32>) -> Result<Vec<HistoryEntry>, String> {
    let lim = limit.unwrap_or(40).clamp(1, 200) as usize;
    Ok(list_sync_history(lim))
}

#[tauri::command]
pub fn get_project_profiles() -> Result<Vec<ProjectProfile>, String> {
    Ok(list_profiles())
}

#[tauri::command]
pub fn upsert_project_profile(
    name: String,
    mode: String,
    this_pc: String,
    brought_over: String,
    replace_id: Option<String>,
) -> Result<ProjectProfile, String> {
    save_profile(name, mode, this_pc, brought_over, replace_id)
}

#[tauri::command]
pub fn remove_project_profile(id: String) -> Result<(), String> {
    delete_profile(id)
}
