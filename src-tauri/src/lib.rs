pub mod engine;

use engine::{
    apply_plan, classify, file_diff, load_snapshot, preview_apply, recover_incomplete,
    save_snapshot, scan_tree, undo_last, ApplyPreview, ApplyReport, Decision, FileDiff, FileRow,
    IgnoreRulesView, ProgressEvent, ScanProgress, ScanResult, SyncRules,
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
    /// Serialize apply / undo across the process.
    apply_lock: Mutex<()>,
    /// Bump when ignore rules change so UI must rescan.
    ignore_epoch: Mutex<u64>,
}

struct SessionData {
    this_pc: PathBuf,
    brought_over: PathBuf,
    rows: Vec<FileRow>,
    ignore_epoch: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            apply_lock: Mutex::new(()),
            ignore_epoch: Mutex::new(0),
        }
    }
}

fn project_opt(path: &Option<String>) -> Option<PathBuf> {
    path.as_ref().filter(|s| !s.is_empty()).map(PathBuf::from)
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
fn recover_journals() -> Result<Vec<ApplyReport>, String> {
    recover_incomplete()
}

#[tauri::command]
fn get_apply_preview(session_id: String, state: State<AppState>) -> Result<ApplyPreview, String> {
    let sessions = state.sessions.lock();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "Session expired. Run a new scan.".to_string())?;
    Ok(preview_apply(&session.rows))
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

    // Recover any crashed applies before scanning.
    let _ = recover_incomplete();

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
    let ignore_epoch = *state.ignore_epoch.lock();

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
                    message: "Scanning Intranet and Internet folders…".into(),
                },
            );
        }
    });

    let session_id_worker = session_id.clone();
    let this_pc_worker = this_pc.clone();
    let brought_over_worker = brought_over.clone();
    let scan_result = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let started = Instant::now();
        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "rules".into(),
                files_seen: 0,
                files_hashed: 0,
                current_path: String::new(),
                message: "Loading sync and ignore rules…".into(),
            },
        );

        let rules = SyncRules::load(&this_path);

        let _ = app.emit(
            "scan-progress",
            ProgressEvent {
                session_id: session_id_worker.clone(),
                phase: "scan-parallel".into(),
                files_seen: 0,
                files_hashed: 0,
                current_path: String::new(),
                message: "Scanning both folders in parallel…".into(),
            },
        );

        // Parallel dual-tree scan
        let progress_a = progress.clone();
        let progress_b = progress.clone();
        let this_path_a = this_path.clone();
        let brought_path_b = brought_path.clone();
        let (this_tree, brought_tree) = std::thread::scope(|scope| {
            let h1 = scope.spawn(|| scan_tree(&this_path_a, &rules, &progress_a));
            let h2 = scope.spawn(|| scan_tree(&brought_path_b, &rules, &progress_b));
            (
                h1.join().expect("this-pc scan"),
                h2.join().expect("brought scan"),
            )
        });

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
                message: "Comparing Intranet vs Internet…".into(),
            },
        );

        let mut snapshot = load_snapshot(&this_path, &brought_path);
        let (rows, mut summary, extras) = classify(
            &this_tree,
            &brought_tree,
            &rules,
            &snapshot,
            &session_id_worker,
        );
        summary.hashed = progress.files_hashed.load(Ordering::Relaxed);
        summary.elapsed_ms = started.elapsed().as_millis() as u64;

        // Upsert identical shared hashes into baseline (does not bump generation).
        if !extras.identical_hashes.is_empty() {
            let mut dirty = false;
            for (path, hash) in &extras.identical_hashes {
                if snapshot.files.get(path) != Some(hash) {
                    snapshot.files.insert(path.clone(), hash.clone());
                    dirty = true;
                }
            }
            if dirty {
                snapshot.this_pc = this_path.display().to_string();
                snapshot.brought_over = brought_path.display().to_string();
                snapshot.updated_at = chrono::Utc::now().to_rfc3339();
                snapshot.version = 2;
                let _ = save_snapshot(&snapshot);
            }
        }

        let mut warnings = Vec::new();
        warnings.extend(rules.warnings.iter().cloned());
        warnings.extend(this_tree.errors.iter().cloned());
        warnings.extend(brought_tree.errors.iter().cloned());
        if let Some(reason) = extras.incomplete_reason {
            warnings.push(reason);
        }
        if snapshot.generation == 0 && snapshot.files.is_empty() {
            warnings.push(
                "First sync for this pair — nothing is written until you accept changes.".into(),
            );
        }

        // Persist last brought count for future incomplete detection
        if !extras.incomplete_brought {
            let mut snap2 = load_snapshot(&this_path, &brought_path);
            snap2.last_brought_file_count = brought_tree.files.len() as u64;
            snap2.this_pc = this_path.display().to_string();
            snap2.brought_over = brought_path.display().to_string();
            let _ = save_snapshot(&snap2);
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
            ignore_epoch,
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
    let epoch = *state.ignore_epoch.lock();
    ensure_ignore_fresh(session, epoch)?;
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
    let epoch = *state.ignore_epoch.lock();
    ensure_ignore_fresh(session, epoch)?;
    for row in &mut session.rows {
        if matches!(row.class, engine::ChangeClass::NeedsYou) {
            row.decision = decision.clone();
        }
    }
    Ok(session.rows.clone())
}

fn ensure_ignore_fresh(session: &SessionData, epoch: u64) -> Result<(), String> {
    if session.ignore_epoch != epoch {
        return Err(
            "Ignore rules changed since this scan. Run a fresh compare before applying.".into(),
        );
    }
    Ok(())
}

#[tauri::command]
fn execute_apply(
    session_id: String,
    confirm_destructive: bool,
    state: State<AppState>,
) -> Result<ApplyReport, String> {
    let _guard = state.apply_lock.lock();
    let epoch = *state.ignore_epoch.lock();
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
    )
}

#[tauri::command]
fn undo_apply(state: State<AppState>) -> Result<ApplyReport, String> {
    let _guard = state.apply_lock.lock();
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

// --- Ignore Rules Manager ---

#[tauri::command]
fn get_ignore_rules(project_path: Option<String>) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    Ok(engine::ignore_config::build_view(root.as_deref()))
}

#[tauri::command]
fn set_ignore_pattern_enabled(
    id: String,
    enabled: bool,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view =
        engine::ignore_config::set_pattern_enabled(&id, enabled, root.as_deref(), project_scope)?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

#[tauri::command]
fn set_ignore_category_enabled(
    category: String,
    enabled: bool,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = engine::ignore_config::set_category_enabled(
        &category,
        enabled,
        root.as_deref(),
        project_scope,
    )?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

#[tauri::command]
fn add_ignore_pattern(
    pattern: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = engine::ignore_config::add_custom_pattern(&pattern, root.as_deref(), project_scope)?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

#[tauri::command]
fn update_ignore_pattern(
    id: String,
    pattern: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view =
        engine::ignore_config::update_pattern(&id, &pattern, root.as_deref(), project_scope)?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

#[tauri::command]
fn delete_ignore_pattern(
    id: String,
    project_path: Option<String>,
    project_scope: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = engine::ignore_config::delete_pattern(&id, root.as_deref(), project_scope)?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

#[tauri::command]
fn export_ignore_rules(project_path: Option<String>) -> Result<String, String> {
    let root = project_opt(&project_path);
    engine::ignore_config::export_config(root.as_deref())
}

#[tauri::command]
fn import_ignore_rules(
    json: String,
    project_path: Option<String>,
    as_project: bool,
    state: State<AppState>,
) -> Result<IgnoreRulesView, String> {
    let root = project_opt(&project_path);
    let view = engine::ignore_config::import_config(&json, root.as_deref(), as_project)?;
    *state.ignore_epoch.lock() += 1;
    Ok(view)
}

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
