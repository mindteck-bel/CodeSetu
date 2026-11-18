use crate::engine::IgnoreRulesView;
use crate::session::AppState;
use std::path::PathBuf;
use tauri::State;

fn project_opt(path: &Option<String>) -> Option<PathBuf> {
    path.as_ref().filter(|s| !s.is_empty()).map(PathBuf::from)
}

#[tauri::command]
pub fn get_ignore_rules(project_path: Option<String>) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    Ok(crate::engine::ignore_config::build_view(root.as_deref()))
}

#[tauri::command]
pub fn set_ignore_pattern_enabled(
    id: String,
    enabled: bool,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = crate::engine::ignore_config::set_pattern_enabled(
        &id,
        enabled,
        root.as_deref(),
        project_scope,
    )?;
    state.bump_ignore_epoch();
    Ok(view)
}

#[tauri::command]
pub fn set_ignore_category_enabled(
    category: String,
    enabled: bool,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = crate::engine::ignore_config::set_category_enabled(
        &category,
        enabled,
        root.as_deref(),
        project_scope,
    )?;
    state.bump_ignore_epoch();
    Ok(view)
}

#[tauri::command]
pub fn add_ignore_pattern(
    pattern: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view =
        crate::engine::ignore_config::add_custom_pattern(&pattern, root.as_deref(), project_scope)?;
    state.bump_ignore_epoch();
    Ok(view)
}

#[tauri::command]
pub fn update_ignore_pattern(
    id: String,
    pattern: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = crate::engine::ignore_config::update_pattern(
        &id,
        &pattern,
        root.as_deref(),
        project_scope,
    )?;
    state.bump_ignore_epoch();
    Ok(view)
}

#[tauri::command]
pub fn delete_ignore_pattern(
    id: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = crate::engine::ignore_config::delete_pattern(&id, root.as_deref(), project_scope)?;
    state.bump_ignore_epoch();
    Ok(view)
}

#[tauri::command]
pub fn export_ignore_rules(project_path: Option<String>) -> Result<String, String> {
    let root = project_opt(&project_path);
    crate::engine::ignore_config::export_config(root.as_deref())
}

#[tauri::command]
pub fn import_ignore_rules(
    json: String,
    project_path: Option<String>,
    as_project: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = crate::engine::ignore_config::import_config(&json, root.as_deref(), as_project)?;
    state.bump_ignore_epoch();
    Ok(view)
}
