mod engine;

use engine::{
    apply_plan, classify, file_diff, load_snapshot, scan_tree, undo_last, ApplyReport, Decision,
    FileDiff, FileRow, ProgressEvent, ScanProgress, ScanResult, SyncRules,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

struct AppState {
    cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
    sessions: Mutex<HashMap<String, SessionData>>,
}

struct SessionData {
    this_pc: PathBuf,
    brought_over: PathBuf,
    rows: Vec<FileRow>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
fn validate_folder(path: String) -> Result<bool, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("Folder does not exist.".into());
    }
    if !p.is_dir() {
        return Err("Path is not a folder.".into());
    }
    // readable check
    match std::fs::read_dir(&p) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Cannot read folder: {e}")),
    }
}

#[tauri::command]
fn cancel_scan(session_id: String, state: State<AppState>) -> Result<(), String> {
    if let Some(flag) = state.cancel.lock().get(&session_id) {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
async fn start_scan(
    app: AppHandle,
    this_pc: String,
    brought_over: String,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let this_path = PathBuf::from(&this_pc);
    let brought_path = PathBuf::from(&brought_over);

    if this_path == brought_path {
        return Err("The two folders must be different.".into());
    }
    validate_folder(this_pc.clone())?;
    validate_folder(brought_over.clone())?;

    let session_id = Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancel
        .lock()
        .insert(session_id.clone(), cancel.clone());

    let progress = ScanProgress::new(cancel.clone());
    let progress_ui = progress.clone();
    let session_for_emit = session_id.clone();
    let app_progress = app.clone();

    // Keep the UI alive: stream progress from a side task while we scan off-thread.
    let ticker = tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if progress_ui.cancel.load(Ordering::Relaxed) {
                break;
            }
            let current = progress_ui.current.lock().clone();
            let _ = app_progress.emit(
                "scan-progress",
                ProgressEvent {
                    session_id: session_for_emit.clone(),
                    phase: "scanning".into(),
                    files_seen: progress_ui.files_seen.load(Ordering::Relaxed),
                    files_hashed: progress_ui.files_hashed.load(Ordering::Relaxed),
                    current_path: current,
                    message: "Scanning…".into(),
                },
            );
        }
    });

    let session_id_worker = session_id.clone();
    let this_pc_worker = this_pc.clone();
    let brought_over_worker = brought_over.clone();
    let scan_result = tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "rules".into(),
                files_seen: 0,
                files_hashed: 0,
                current_path: String::new(),
                message: "Loading sync rules…".into(),
            },
        );

        let rules = SyncRules::load(&this_path);

        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "scan-this".into(),
                files_seen: progress.files_seen.load(Ordering::Relaxed),
                files_hashed: progress.files_hashed.load(Ordering::Relaxed),
                current_path: String::new(),
                message: "Scanning this PC…".into(),
            },
        );
        let this_tree = scan_tree(&this_path, &rules, &progress);
        if cancel.load(Ordering::SeqCst) {
            return Err("Scan cancelled.".into());
        }

        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "scan-brought".into(),
                files_seen: progress.files_seen.load(Ordering::Relaxed),
                files_hashed: progress.files_hashed.load(Ordering::Relaxed),
                current_path: String::new(),
                message: "Scanning brought-over folder…".into(),
            },
        );
        let brought_tree = scan_tree(&brought_path, &rules, &progress);
        if cancel.load(Ordering::SeqCst) {
            return Err("Scan cancelled.".into());
        }

        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "classify".into(),
                files_seen: progress.files_seen.load(Ordering::Relaxed),
                files_hashed: progress.files_hashed.load(Ordering::Relaxed),
                current_path: String::new(),
                message: "Comparing changes…".into(),
            },
        );

        let snapshot = load_snapshot(&this_path, &brought_path);
        let (rows, mut summary) =
            classify(&this_tree, &brought_tree, &rules, &snapshot, &session_id_worker);
        summary.hashed = progress.files_hashed.load(Ordering::Relaxed);
        summary.elapsed_ms = started.elapsed().as_millis() as u64;

        let mut warnings = Vec::new();
        warnings.extend(this_tree.errors.iter().cloned());
        warnings.extend(brought_tree.errors.iter().cloned());
        if snapshot.files.is_empty() {
            warnings.push(
                "First sync for this pair — nothing is written until you accept changes.".into(),
            );
        }

        progress.cancel.store(true, Ordering::SeqCst);

        Ok((
            ScanResult {
                session_id: session_id_worker,
                this_pc: this_pc_worker,
                brought_over: brought_over_worker,
                summary,
                rows: rows.clone(),
                warnings,
            },
            this_path,
            brought_path,
            rows,
        ))
    })
    .await
    .map_err(|e| format!("Scan worker failed: {e}"))?;

    let _ = ticker.await;

    let (result, this_path, brought_path, rows) = scan_result?;

    state.sessions.lock().insert(
        result.session_id.clone(),
        SessionData {
            this_pc: this_path,
            brought_over: brought_path,
            rows,
        },
    );

    Ok(result)
}

#[tauri::command]
fn update_decisions(
    session_id: String,
    updates: Vec<(String, Decision)>,
    state: State<AppState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    let map: HashMap<_, _> = updates.into_iter().collect();
    for row in &mut session.rows {
        if let Some(d) = map.get(&row.id) {
            row.decision = d.clone();
        }
    }
    Ok(())
}

#[tauri::command]
fn set_all_needs_you(
    session_id: String,
    decision: Decision,
    state: State<AppState>,
) -> Result<Vec<FileRow>, String> {
    let mut sessions = state.sessions.lock();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    for row in &mut session.rows {
        if matches!(row.class, engine::ChangeClass::NeedsYou) {
            row.decision = decision.clone();
        }
    }
    Ok(session.rows.clone())
}

#[tauri::command]
fn execute_apply(session_id: String, state: State<AppState>) -> Result<ApplyReport, String> {
    let sessions = state.sessions.lock();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    let snapshot = load_snapshot(&session.this_pc, &session.brought_over);
    apply_plan(
        &session.this_pc,
        &session.brought_over,
        &session.rows,
        &session_id,
        snapshot,
    )
}

#[tauri::command]
fn undo_apply() -> Result<ApplyReport, String> {
    undo_last()
}

#[tauri::command]
fn get_file_diff(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodeSetu");
}
