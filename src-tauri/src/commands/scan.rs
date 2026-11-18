use crate::commands::folders::validate_folder;
use crate::engine::{
    classify, diag, load_snapshot, recover_incomplete, save_snapshot, scan_tree, ProgressEvent,
    ScanProgress, ScanResult, SyncRules,
};
use crate::session::{AppState, SessionData};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[tauri::command]
pub fn cancel_scan(session_id: String, state: State<AppState>) -> Result<(), String> {
    if let Some(flag) = state.cancel.lock().get(&session_id) {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub async fn start_scan(
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
    let ignore_epoch = state.ignore_epoch();

    diag::log_event(
        "scan.start",
        json!({
            "sessionId": session_id,
            "thisPc": this_pc,
            "broughtOver": brought_over,
        }),
    );

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
        let first_sync = snapshot.generation == 0 && snapshot.files.is_empty();
        let (rows, mut summary, extras) = classify(
            &this_tree,
            &brought_tree,
            &rules,
            &snapshot,
            &session_id_worker,
        );
        summary.hashed = progress.files_hashed.load(Ordering::Relaxed);
        summary.elapsed_ms = started.elapsed().as_millis() as u64;
        summary.first_sync = first_sync;

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
        if first_sync {
            warnings.push(
                "First sync for this pair — review new files carefully; nothing is written until you confirm apply.".into(),
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

        diag::log_event(
            "scan.done",
            json!({
                "sessionId": session_id_worker,
                "elapsedMs": summary.elapsed_ms,
                "hashed": summary.hashed,
                "needsYou": summary.needs_you,
                "firstSync": first_sync,
                "warnings": warnings.len(),
            }),
        );

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
            first_sync,
        ))
    })
    .await
    .map_err(|e| format!("Scan worker failed: {e}"))?;

    let _ = ticker.await;

    let (result, this_path, brought_path, rows, first_sync) = scan_result?;

    state.sessions.lock().insert(
        result.session_id.clone(),
        SessionData {
            this_pc: this_path,
            brought_over: brought_path,
            rows,
            ignore_epoch,
            first_sync,
        },
    );

    Ok(result)
}
