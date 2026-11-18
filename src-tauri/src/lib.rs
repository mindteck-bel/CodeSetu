//! CodeSetu Tauri host — thin command registration over `engine` + `session`.

pub mod commands;
pub mod engine;
pub mod session;

use commands::*;
use engine::recover_incomplete;
use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Best-effort recovery at startup
    let _ = recover_incomplete();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            validate_folder,
            cancel_scan,
            start_scan,
            update_decisions,
            set_all_needs_you,
            execute_apply,
            undo_apply,
            get_file_diff,
            get_apply_preview,
            recover_journals,
            get_ignore_rules,
            set_ignore_pattern_enabled,
            set_ignore_category_enabled,
            add_ignore_pattern,
            update_ignore_pattern,
            delete_ignore_pattern,
            export_ignore_rules,
            import_ignore_rules,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodeSetu");
}
