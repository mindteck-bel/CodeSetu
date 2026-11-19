use crate::engine::{
    apply_plan, file_diff, load_snapshot, preview_apply, recover_incomplete, undo_last,
    ApplyPreview, ApplyReport, Decision, FileDiff, FileRow,
};
use crate::session::{ensure_ignore_fresh, AppState};
use tauri::State;

#[tauri::command]
pub fn recover_journals() -> Result<Vec<ApplyReport>, String> {
    recover_incomplete()
}

#[tauri::command]
pub fn get_apply_preview(
    session_id: String,
    only_ids: Option<Vec<String>>,
    state: State<AppState>,
) -> Result<ApplyPreview, String> {
    let sessions = state.sessions.lock();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    Ok(preview_apply(
        &session.rows,
        session.first_sync,
        only_ids.as_deref(),
    ))
}

#[tauri::command]
pub fn update_decisions(
    session_id: String,
    updates: Vec<(String, Decision)>,
    state: State<AppState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    let epoch = state.ignore_epoch();
    ensure_ignore_fresh(session, epoch)?;
    let map: std::collections::HashMap<_, _> = updates.into_iter().collect();
    for row in &mut session.rows {
        if let Some(d) = map.get(&row.id) {
            row.decision = d.clone();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_all_needs_you(
    session_id: String,
    decision: Decision,
    only_ids: Option<Vec<String>>,
    state: State<AppState>,
) -> Result<Vec<FileRow>, String> {
    let mut sessions = state.sessions.lock();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    let epoch = state.ignore_epoch();
    ensure_ignore_fresh(session, epoch)?;
    if session.first_sync && matches!(decision, Decision::BringIn) && only_ids.is_none() {
        return Err(
            "First sync: resolve conflicts one-by-one or confirm apply — bulk Use source is disabled to avoid overwriting unexpected trees."
                .into(),
        );
    }
    let filter: Option<std::collections::HashSet<String>> =
        only_ids.map(|ids| ids.into_iter().collect());
    for row in &mut session.rows {
        let in_scope = match &filter {
            None => matches!(row.class, crate::engine::ChangeClass::NeedsYou),
            Some(ids) => {
                ids.contains(&row.id)
                    && !matches!(
                        row.class,
                        crate::engine::ChangeClass::IntranetOnly
                            | crate::engine::ChangeClass::InternetOnly
                            | crate::engine::ChangeClass::ParkedSecret
                    )
            }
        };
        if in_scope {
            row.decision = decision.clone();
        }
    }
    Ok(session.rows.clone())
}

#[tauri::command]
pub fn execute_apply(
    session_id: String,
    confirm_destructive: bool,
    only_ids: Option<Vec<String>>,
    state: State<AppState>,
) -> Result<ApplyReport, String> {
    let _guard = state.apply_lock.lock();
    let epoch = state.ignore_epoch();
    let (this_pc, brought_over, rows) = {
        let sessions = state.sessions.lock();
        let session = sessions
            .get(&session_id)
            .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
        ensure_ignore_fresh(session, epoch)?;
        (
            session.this_pc.clone(),
            session.brought_over.clone(),
            session.rows.clone(),
        )
    };
    let snapshot = load_snapshot(&this_pc, &brought_over);
    apply_plan(
        &this_pc,
        &brought_over,
        &rows,
        &session_id,
        snapshot,
        true,
        confirm_destructive,
        only_ids.as_deref(),
    )
}

#[tauri::command]
pub fn undo_apply(state: State<AppState>) -> Result<ApplyReport, String> {
    let _guard = state.apply_lock.lock();
    undo_last()
}

#[tauri::command]
pub fn get_file_diff(
    session_id: String,
    rel_path: String,
    state: State<AppState>,
) -> Result<FileDiff, String> {
    let sessions = state.sessions.lock();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    Ok(file_diff(
        &session.this_pc,
        &session.brought_over,
        &rel_path,
    ))
}
