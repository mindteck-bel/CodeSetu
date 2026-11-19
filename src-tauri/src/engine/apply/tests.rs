use super::*;
use crate::engine::hash::hash_file;
use crate::engine::snapshot::{save_snapshot, save_snapshot_to, Snapshot};
use crate::engine::types::{ChangeClass, Decision, FileKind, FileRow, PathZone};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn row(
    rel: &str,
    class: ChangeClass,
    decision: Decision,
    this_hash: Option<&str>,
    brought_hash: Option<&str>,
    moved_from: Option<&str>,
) -> FileRow {
    FileRow {
        id: rel.into(),
        rel_path: rel.into(),
        group: "src".into(),
        class,
        decision,
        zone: PathZone::Shared,
        kind: FileKind::Text,
        this_size: None,
        brought_size: None,
        this_hash: this_hash.map(|s| s.to_string()),
        brought_hash: brought_hash.map(|s| s.to_string()),
        moved_from: moved_from.map(|s| s.to_string()),
        error: None,
    }
}

fn with_data_dir<T>(f: impl FnOnce(&Path) -> T) -> T {
    let dir = tempdir().unwrap();
    crate::engine::paths::with_test_data_dir(dir.path(), || f(dir.path()))
}

#[test]
fn copies_bring_in_and_skips_intranet_only() {
    with_data_dir(|data| {
        let this_pc = data.join("intra");
        let brought = data.join("inet");
        fs::create_dir_all(this_pc.join("src")).unwrap();
        fs::create_dir_all(brought.join("src")).unwrap();
        fs::create_dir_all(this_pc.join("deploy/intra")).unwrap();
        fs::create_dir_all(brought.join("deploy/intra")).unwrap();
        fs::write(this_pc.join("src/a.txt"), b"old").unwrap();
        fs::write(brought.join("src/a.txt"), b"new").unwrap();
        fs::write(this_pc.join("deploy/intra/x.yml"), b"keep-me").unwrap();
        fs::write(brought.join("deploy/intra/x.yml"), b"ignore").unwrap();

        let (old_hash, _) = hash_file(&this_pc.join("src/a.txt")).unwrap();
        let (new_hash, _) = hash_file(&brought.join("src/a.txt")).unwrap();

        let rows = vec![
            row(
                "src/a.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&old_hash),
                Some(&new_hash),
                None,
            ),
            FileRow {
                id: "2".into(),
                rel_path: "deploy/intra/x.yml".into(),
                group: "deploy".into(),
                class: ChangeClass::IntranetOnly,
                decision: Decision::Keep,
                zone: PathZone::IntranetOnly,
                kind: FileKind::Text,
                this_size: Some(7),
                brought_size: Some(6),
                this_hash: None,
                brought_hash: None,
                moved_from: None,
                error: None,
            },
        ];

        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        // Non-empty baseline so first-sync gate does not fire for this overwrite test.
        snap.files.insert("baseline.txt".into(), "x".into());
        snap.generation = 1;
        let report = apply_plan(
            &this_pc,
            &brought,
            &rows,
            "test-session",
            snap,
            false,
            false,
            None,
        )
        .unwrap();
        assert_eq!(report.applied, 1);
        assert_eq!(
            fs::read_to_string(this_pc.join("src/a.txt")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(this_pc.join("deploy/intra/x.yml")).unwrap(),
            "keep-me"
        );
    });
}

#[test]
fn overwrites_existing_file() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(this_pc.join("f.txt"), b"old").unwrap();
        fs::write(brought.join("f.txt"), b"new-content").unwrap();
        let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
        let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();
        let rows = vec![row(
            "f.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            Some(&th),
            Some(&bh),
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.files.insert("f.txt".into(), th.clone());
        snap.generation = 1;
        apply_plan(&this_pc, &brought, &rows, "ow", snap, false, false, None).unwrap();
        assert_eq!(
            fs::read_to_string(this_pc.join("f.txt")).unwrap(),
            "new-content"
        );
    });
}

#[test]
fn undo_restores_files_and_snapshot() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(this_pc.join("f.txt"), b"local").unwrap();
        fs::write(brought.join("f.txt"), b"remote").unwrap();
        let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
        let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();

        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.files.insert("f.txt".into(), th.clone());
        snap.generation = 1;
        save_snapshot(&snap).unwrap();

        let rows = vec![row(
            "f.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            Some(&th),
            Some(&bh),
            None,
        )];
        apply_plan(
            &this_pc,
            &brought,
            &rows,
            "undo-sess",
            snap.clone(),
            false,
            false,
            None,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "remote");

        let after = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
        assert_eq!(after.files.get("f.txt").unwrap(), &bh);

        undo_last().unwrap();
        assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "local");
        let restored = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
        assert_eq!(restored.files.get("f.txt").unwrap(), &th);
        assert_eq!(restored.generation, 1);
    });
}

#[test]
fn move_undo_restores_both_paths() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(this_pc.join("old")).unwrap();
        fs::create_dir_all(brought.join("new")).unwrap();
        fs::write(this_pc.join("old/a.txt"), b"same-bytes").unwrap();
        fs::write(brought.join("new/a.txt"), b"same-bytes").unwrap();
        let (h, _) = hash_file(&brought.join("new/a.txt")).unwrap();
        let rows = vec![row(
            "new/a.txt",
            ChangeClass::Moved,
            Decision::BringIn,
            None,
            Some(&h),
            Some("old/a.txt"),
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.files.insert("old/a.txt".into(), h.clone());
        snap.generation = 1;
        apply_plan(
            &this_pc,
            &brought,
            &rows,
            "move-sess",
            snap,
            false,
            false,
            None,
        )
        .unwrap();
        assert!(!this_pc.join("old/a.txt").exists());
        assert!(this_pc.join("new/a.txt").exists());
        undo_last().unwrap();
        assert!(this_pc.join("old/a.txt").exists());
        assert!(!this_pc.join("new/a.txt").exists());
    });
}

#[test]
fn crash_mid_apply_recovers() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(this_pc.join("a.txt"), b"A0").unwrap();
        fs::write(brought.join("a.txt"), b"A1").unwrap();
        let (th, _) = hash_file(&this_pc.join("a.txt")).unwrap();
        let (bh, _) = hash_file(&brought.join("a.txt")).unwrap();

        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.files.insert("a.txt".into(), th.clone());
        save_snapshot(&snap).unwrap();

        // Simulate partial apply: write inprogress journal with one entry already applied.
        let sid = "crash-sess";
        let backups = journal::backup_dir(sid);
        fs::create_dir_all(&backups).unwrap();
        let pre = backups.join("pre-apply-snapshot.json");
        save_snapshot_to(&pre, &snap).unwrap();
        let backup = backups.join("a.txt");
        fs::copy(this_pc.join("a.txt"), &backup).unwrap();
        fs::write(this_pc.join("a.txt"), b"A1").unwrap();

        let journal = journal::Journal {
            session_id: sid.into(),
            this_pc: this_pc.display().to_string(),
            brought_over: brought.display().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            status: journal::JournalStatus::InProgress,
            pre_snapshot_path: Some(pre.display().to_string()),
            entries: vec![journal::JournalEntry {
                rel_path: "a.txt".into(),
                action: "copy".into(),
                backup_path: Some(backup.display().to_string()),
                previous_hash: Some(th.clone()),
                new_hash: Some(bh),
                moved_from: None,
                moved_from_backup: None,
            }],
        };
        fs::create_dir_all(journal::journal_dir()).unwrap();
        journal::persist_journal(&journal, &journal::inprogress_path(sid)).unwrap();

        let reports = recover_incomplete().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(fs::read_to_string(this_pc.join("a.txt")).unwrap(), "A0");
        let restored = crate::engine::snapshot::load_snapshot(&this_pc, &brought);
        assert_eq!(restored.files.get("a.txt").unwrap(), &th);
    });
}

#[test]
fn rejects_changed_source_on_revalidate() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(this_pc.join("f.txt"), b"local").unwrap();
        fs::write(brought.join("f.txt"), b"remote-now").unwrap();
        let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
        let rows = vec![row(
            "f.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            Some(&th),
            Some("deadbeef"),
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.generation = 1;
        snap.files.insert("f.txt".into(), th.clone());
        let report =
            apply_plan(&this_pc, &brought, &rows, "reval", snap, false, false, None).unwrap();
        assert_eq!(report.failed, 1);
        assert_eq!(fs::read_to_string(this_pc.join("f.txt")).unwrap(), "local");
    });
}

#[test]
fn path_traversal_rejected() {
    with_data_dir(|data| {
        let this_pc = data.join("intra");
        let brought = data.join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        let rows = vec![row(
            "../escape.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            None,
            None,
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.generation = 1;
        snap.files.insert("x".into(), "y".into());
        let report =
            apply_plan(&this_pc, &brought, &rows, "trav", snap, false, false, None).unwrap();
        assert_eq!(report.failed, 1);
    });
}

#[test]
fn destructive_confirm_required() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(this_pc.join("f.txt"), b"x").unwrap();
        fs::write(brought.join("f.txt"), b"y").unwrap();
        let (th, _) = hash_file(&this_pc.join("f.txt")).unwrap();
        let (bh, _) = hash_file(&brought.join("f.txt")).unwrap();
        let rows = vec![row(
            "f.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            Some(&th),
            Some(&bh),
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.generation = 1;
        snap.files.insert("f.txt".into(), th.clone());
        let err =
            apply_plan(&this_pc, &brought, &rows, "conf", snap, true, false, None).unwrap_err();
        assert!(err.contains("confirmation"));
    });
}

#[test]
fn first_sync_confirm_required_for_creates() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(&this_pc).unwrap();
        fs::create_dir_all(&brought).unwrap();
        fs::write(brought.join("new.txt"), b"hello").unwrap();
        let (bh, _) = hash_file(&brought.join("new.txt")).unwrap();
        let rows = vec![row(
            "new.txt",
            ChangeClass::BringIn,
            Decision::BringIn,
            None,
            Some(&bh),
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        let err =
            apply_plan(&this_pc, &brought, &rows, "fsync", snap, true, false, None).unwrap_err();
        assert!(err.to_lowercase().contains("first sync"));
    });
}

#[test]
fn unusual_filename_roundtrip() {
    with_data_dir(|_| {
        let dir = tempdir().unwrap();
        let this_pc = dir.path().join("intra");
        let brought = dir.path().join("inet");
        fs::create_dir_all(brought.join("docs")).unwrap();
        fs::create_dir_all(this_pc.join("docs")).unwrap();
        let name = "docs/weird name (1) — café.txt";
        fs::write(brought.join(name), b"unicode-ok").unwrap();
        let (bh, _) = hash_file(&brought.join(name)).unwrap();
        let rows = vec![row(
            name,
            ChangeClass::BringIn,
            Decision::BringIn,
            None,
            Some(&bh),
            None,
        )];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        apply_plan(&this_pc, &brought, &rows, "weird", snap, false, true, None).unwrap();
        assert_eq!(
            fs::read_to_string(this_pc.join(name)).unwrap(),
            "unicode-ok"
        );
    });
}

#[test]
fn selective_apply_only_mutates_selected_ids() {
    with_data_dir(|data| {
        let this_pc = data.join("intra");
        let brought = data.join("inet");
        fs::create_dir_all(this_pc.join("src")).unwrap();
        fs::create_dir_all(brought.join("src")).unwrap();
        fs::write(this_pc.join("src/a.txt"), b"old-a").unwrap();
        fs::write(this_pc.join("src/b.txt"), b"old-b").unwrap();
        fs::write(brought.join("src/a.txt"), b"new-a").unwrap();
        fs::write(brought.join("src/b.txt"), b"new-b").unwrap();
        let (oa, _) = hash_file(&this_pc.join("src/a.txt")).unwrap();
        let (ob, _) = hash_file(&this_pc.join("src/b.txt")).unwrap();
        let (na, _) = hash_file(&brought.join("src/a.txt")).unwrap();
        let (nb, _) = hash_file(&brought.join("src/b.txt")).unwrap();
        let rows = vec![
            row(
                "src/a.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&oa),
                Some(&na),
                None,
            ),
            row(
                "src/b.txt",
                ChangeClass::BringIn,
                Decision::BringIn,
                Some(&ob),
                Some(&nb),
                None,
            ),
        ];
        let mut snap = Snapshot::default();
        snap.this_pc = this_pc.display().to_string();
        snap.brought_over = brought.display().to_string();
        snap.files.insert("baseline.txt".into(), "x".into());
        snap.generation = 1;
        let only = vec!["src/a.txt".into()];
        let preview = preview_apply(&rows, false, Some(&only));
        assert_eq!(preview.rows_to_apply, 1);
        assert_eq!(preview.overwrites, 1);
        assert_eq!(preview.selected_count, 1);
        let report = apply_plan(
            &this_pc,
            &brought,
            &rows,
            "sel-sess",
            snap,
            false,
            false,
            Some(&only),
        )
        .unwrap();
        assert_eq!(report.applied, 1);
        assert_eq!(
            fs::read_to_string(this_pc.join("src/a.txt")).unwrap(),
            "new-a"
        );
        assert_eq!(
            fs::read_to_string(this_pc.join("src/b.txt")).unwrap(),
            "old-b"
        );
    });
}
