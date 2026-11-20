use crate::engine::types::*;
use crate::engine::Snapshot;
use crate::engine::SyncRules;
use std::collections::HashMap;
use uuid::Uuid;

/// Completeness threshold: if brought tree has fewer than this fraction of the
/// last known brought file count (from snapshot), treat as incomplete and never
/// emit mass deletions.
pub const INCOMPLETE_RATIO: f64 = 0.70;

#[derive(Debug, Clone, Default)]
pub struct ClassifyExtras {
    pub incomplete_brought: bool,
    pub incomplete_reason: Option<String>,
    /// Identical shared paths that should be upserted into the snapshot baseline.
    pub identical_hashes: HashMap<String, String>,
}

pub fn classify(
    this_tree: &TreeIndex,
    brought_tree: &TreeIndex,
    rules: &SyncRules,
    snapshot: &Snapshot,
    session_id: &str,
) -> (Vec<FileRow>, ScanSummary, ClassifyExtras) {
    let mut summary = ScanSummary {
        total_this: this_tree.files.len() as u64,
        total_brought: brought_tree.files.len() as u64,
        errors: (this_tree.errors.len() + brought_tree.errors.len()) as u64,
        ..Default::default()
    };
    let mut extras = ClassifyExtras::default();

    // Incomplete-copy protection: never propagate deletions from unverified trees.
    let incomplete = detect_incomplete(brought_tree, snapshot);
    if let Some(reason) = incomplete {
        extras.incomplete_brought = true;
        extras.incomplete_reason = Some(reason);
    }

    let mut rows: Vec<FileRow> = Vec::new();

    // Move detection only when hash is unique on both sides.
    let this_by_hash: HashMap<&str, Vec<&str>> = invert_hash_index(&this_tree.files);
    let brought_by_hash: HashMap<&str, Vec<&str>> = invert_hash_index(&brought_tree.files);

    let mut all_paths: Vec<String> = this_tree
        .files
        .keys()
        .chain(brought_tree.files.keys())
        .cloned()
        .collect();
    all_paths.sort();
    all_paths.dedup();

    for rel in all_paths {
        let zone = rules.zone(&rel);
        if zone == PathZone::Ignored {
            continue;
        }

        let this_f = this_tree.files.get(&rel);
        let brought_f = brought_tree.files.get(&rel);
        let snap = snapshot.files.get(&rel);

        let group = group_of(&rel);

        match zone {
            PathZone::IntranetOnly => {
                summary.intranet_only += 1;
                if this_f.is_some() {
                    rows.push(row(
                        session_id,
                        &rel,
                        &group,
                        ChangeClass::IntranetOnly,
                        Decision::Keep,
                        zone,
                        this_f,
                        brought_f,
                        None,
                    ));
                }
                continue;
            }
            PathZone::InternetOnly => {
                summary.internet_only += 1;
                if brought_f.is_some() {
                    rows.push(row(
                        session_id,
                        &rel,
                        &group,
                        ChangeClass::InternetOnly,
                        Decision::Skip,
                        zone,
                        this_f,
                        brought_f,
                        None,
                    ));
                }
                continue;
            }
            PathZone::ParkedSecret => {
                summary.parked_secret += 1;
                rows.push(row(
                    session_id,
                    &rel,
                    &group,
                    ChangeClass::ParkedSecret,
                    Decision::Skip,
                    zone,
                    this_f,
                    brought_f,
                    None,
                ));
                continue;
            }
            PathZone::Ignored => continue,
            PathZone::Shared => {}
        }

        match (this_f, brought_f) {
            (Some(t), Some(b)) if t.hash == b.hash => {
                summary.identical += 1;
                // Record identical shared files so the baseline stays complete.
                extras.identical_hashes.insert(rel.clone(), t.hash.clone());
            }
            (Some(t), Some(b)) => {
                let class = if let Some(s) = snap {
                    if s == &t.hash && s != &b.hash {
                        ChangeClass::BringIn
                    } else if s == &b.hash && s != &t.hash {
                        ChangeClass::Keep
                    } else {
                        ChangeClass::NeedsYou
                    }
                } else {
                    ChangeClass::NeedsYou
                };

                // Conflicts require an explicit Use source / Keep mine choice.
                // Defaulting to Keep made Apply report "skipped" while quietly
                // locking the destination version into the snapshot.
                let decision = match class {
                    ChangeClass::BringIn => Decision::BringIn,
                    ChangeClass::Keep => Decision::Keep,
                    ChangeClass::NeedsYou => Decision::Skip,
                    _ => Decision::Skip,
                };

                match class {
                    ChangeClass::BringIn => summary.bring_in += 1,
                    ChangeClass::Keep => summary.keep += 1,
                    ChangeClass::NeedsYou => summary.needs_you += 1,
                    _ => {}
                }

                let mut file_row = row(
                    session_id,
                    &rel,
                    &group,
                    class.clone(),
                    decision,
                    zone,
                    Some(t),
                    Some(b),
                    None,
                );
                if matches!(class, ChangeClass::NeedsYou) {
                    file_row.error = Some(conflict_reason(snap, &t.hash, &b.hash));
                }
                rows.push(file_row);
            }
            (None, Some(b)) => {
                // Move: unique hash on both sides, old path missing on brought.
                if this_by_hash.get(b.hash.as_str()).map(|v| v.len()) == Some(1)
                    && brought_by_hash.get(b.hash.as_str()).map(|v| v.len()) == Some(1)
                {
                    if let Some(old_paths) = this_by_hash.get(b.hash.as_str()) {
                        if let Some(old) = old_paths
                            .iter()
                            .find(|p| !brought_tree.files.contains_key(**p))
                        {
                            summary.moved += 1;
                            rows.push(row(
                                session_id,
                                &rel,
                                &group,
                                ChangeClass::Moved,
                                Decision::BringIn,
                                zone,
                                None,
                                Some(b),
                                Some((*old).to_string()),
                            ));
                            continue;
                        }
                    }
                }
                summary.bring_in += 1;
                rows.push(row(
                    session_id,
                    &rel,
                    &group,
                    ChangeClass::BringIn,
                    Decision::BringIn,
                    zone,
                    None,
                    Some(b),
                    None,
                ));
            }
            (Some(t), None) => {
                // Move handled from added side when unique.
                if this_by_hash.get(t.hash.as_str()).map(|v| v.len()) == Some(1)
                    && brought_by_hash.get(t.hash.as_str()).map(|v| v.len()) == Some(1)
                {
                    if let Some(new_paths) = brought_by_hash.get(t.hash.as_str()) {
                        if new_paths.iter().any(|p| !this_tree.files.contains_key(*p)) {
                            continue;
                        }
                    }
                }

                if snapshot.keep_despite_absent.contains(&rel) {
                    summary.keep += 1;
                    rows.push(row(
                        session_id,
                        &rel,
                        &group,
                        ChangeClass::Keep,
                        Decision::Keep,
                        zone,
                        Some(t),
                        None,
                        None,
                    ));
                    continue;
                }

                if snap.map(|s| s == &t.hash).unwrap_or(false) {
                    if extras.incomplete_brought {
                        // Never emit deletions from incomplete/unverified trees.
                        summary.keep += 1;
                        rows.push(row(
                            session_id,
                            &rel,
                            &group,
                            ChangeClass::Keep,
                            Decision::Keep,
                            zone,
                            Some(t),
                            None,
                            None,
                        ));
                    } else {
                        summary.deleted_brought += 1;
                        rows.push(row(
                            session_id,
                            &rel,
                            &group,
                            ChangeClass::DeletedBrought,
                            Decision::Skip,
                            zone,
                            Some(t),
                            None,
                            None,
                        ));
                    }
                } else {
                    summary.keep += 1;
                    rows.push(row(
                        session_id,
                        &rel,
                        &group,
                        ChangeClass::Keep,
                        Decision::Keep,
                        zone,
                        Some(t),
                        None,
                        None,
                    ));
                }
            }
            (None, None) => {}
        }
    }

    let _ = session_id;
    rows.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    (rows, summary, extras)
}

fn detect_incomplete(brought_tree: &TreeIndex, snapshot: &Snapshot) -> Option<String> {
    let actual = brought_tree.files.len() as u64;
    if snapshot.last_brought_file_count > 0 {
        let expected = snapshot.last_brought_file_count;
        if actual < ((expected as f64) * INCOMPLETE_RATIO) as u64 {
            return Some(format!(
                "Source tree looks incomplete ({actual} files vs last known {expected}). Deletions suppressed."
            ));
        }
    } else if !snapshot.files.is_empty() {
        let expected = snapshot.files.len() as u64;
        if expected >= 20 && actual < ((expected as f64) * INCOMPLETE_RATIO) as u64 {
            return Some(format!(
                "Source tree has far fewer files ({actual}) than the last baseline ({expected}). Deletions suppressed — verify the source folder is a complete project copy."
            ));
        }
    }
    None
}

fn invert_hash_index(files: &HashMap<String, FileEntry>) -> HashMap<&str, Vec<&str>> {
    let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
    for (path, entry) in files {
        map.entry(entry.hash.as_str())
            .or_default()
            .push(path.as_str());
    }
    map
}

fn group_of(rel: &str) -> String {
    rel.split('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("(root)")
        .to_string()
}

fn conflict_reason(snap: Option<&String>, this_hash: &str, brought_hash: &str) -> String {
    match snap {
        None => "Both sides differ and there is no prior baseline — choose which version to keep."
            .into(),
        Some(s) if s != this_hash && s != brought_hash => {
            "Both Intranet and Internet changed since the last accepted sync.".into()
        }
        Some(_) => "Versions diverge from the last accepted baseline — resolve manually.".into(),
    }
}

fn row(
    _session: &str,
    rel: &str,
    group: &str,
    class: ChangeClass,
    decision: Decision,
    zone: PathZone,
    this_f: Option<&FileEntry>,
    brought_f: Option<&FileEntry>,
    moved_from: Option<String>,
) -> FileRow {
    FileRow {
        id: Uuid::new_v4().to_string(),
        rel_path: rel.to_string(),
        group: group.to_string(),
        class,
        decision,
        zone,
        kind: this_f
            .or(brought_f)
            .map(|f| f.kind.clone())
            .unwrap_or(FileKind::Text),
        this_size: this_f.map(|f| f.size),
        brought_size: brought_f.map(|f| f.size),
        this_hash: this_f.map(|f| f.hash.clone()),
        brought_hash: brought_f.map(|f| f.hash.clone()),
        moved_from,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::hash::hash_file;
    use std::fs;
    use tempfile::tempdir;

    fn entry(dir: &std::path::Path, rel: &str, body: &[u8]) -> FileEntry {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, body).unwrap();
        let (hash, size) = hash_file(&p).unwrap();
        FileEntry {
            rel_path: rel.to_string(),
            size,
            hash,
            kind: FileKind::Text,
        }
    }

    #[test]
    fn first_sync_conflict_needs_you() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let mut brought = TreeIndex::default();
            this_tree
                .files
                .insert("a.txt".into(), entry(dir.path(), "a-this.txt", b"local"));
            let e = this_tree.files.get_mut("a.txt").unwrap();
            e.rel_path = "a.txt".into();
            brought.files.insert(
                "a.txt".into(),
                entry(dir.path(), "a-brought.txt", b"remote"),
            );
            let e = brought.files.get_mut("a.txt").unwrap();
            e.rel_path = "a.txt".into();

            let rules = SyncRules::load(dir.path());
            let snap = Snapshot::default();
            let (rows, summary, _) = classify(&this_tree, &brought, &rules, &snap, "s1");
            assert_eq!(summary.needs_you, 1);
            assert_eq!(rows[0].class, ChangeClass::NeedsYou);
            assert_eq!(rows[0].decision, Decision::Skip);
        });
    }

    #[test]
    fn snapshot_makes_bring_in_automatic() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let mut brought = TreeIndex::default();
            let local = entry(dir.path(), "l.txt", b"same-base");
            let remote = entry(dir.path(), "r.txt", b"new-on-internet");
            this_tree.files.insert(
                "svc/a.txt".into(),
                FileEntry {
                    rel_path: "svc/a.txt".into(),
                    size: local.size,
                    hash: local.hash.clone(),
                    kind: FileKind::Text,
                },
            );
            brought.files.insert(
                "svc/a.txt".into(),
                FileEntry {
                    rel_path: "svc/a.txt".into(),
                    size: remote.size,
                    hash: remote.hash.clone(),
                    kind: FileKind::Text,
                },
            );
            let mut snap = Snapshot::default();
            snap.files.insert("svc/a.txt".into(), local.hash);
            let rules = SyncRules::load(dir.path());
            let (rows, summary, _) = classify(&this_tree, &brought, &rules, &snap, "s1");
            assert_eq!(summary.bring_in, 1);
            assert_eq!(rows[0].class, ChangeClass::BringIn);
            assert_eq!(rows[0].decision, Decision::BringIn);
        });
    }

    #[test]
    fn identical_recorded_in_extras() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let mut brought = TreeIndex::default();
            let e = entry(dir.path(), "same.txt", b"identical");
            this_tree.files.insert("same.txt".into(), e.clone());
            brought.files.insert("same.txt".into(), e);
            let rules = SyncRules::load(dir.path());
            let (_, summary, extras) =
                classify(&this_tree, &brought, &rules, &Snapshot::default(), "s1");
            assert_eq!(summary.identical, 1);
            assert!(extras.identical_hashes.contains_key("same.txt"));
        });
    }

    #[test]
    fn incomplete_tree_suppresses_deletions() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let brought = TreeIndex::default(); // empty / incomplete
            let mut snap = Snapshot::default();
            for i in 0..40 {
                let name = format!("f{i}.txt");
                let e = entry(
                    dir.path(),
                    &format!("t{i}.txt"),
                    format!("body{i}").as_bytes(),
                );
                snap.files.insert(name.clone(), e.hash.clone());
                this_tree.files.insert(name, e);
            }
            snap.last_brought_file_count = 40;
            let rules = SyncRules::load(dir.path());
            let (rows, summary, extras) = classify(&this_tree, &brought, &rules, &snap, "s1");
            assert!(extras.incomplete_brought);
            assert_eq!(summary.deleted_brought, 0);
            assert!(rows.iter().all(|r| r.class != ChangeClass::DeletedBrought));
        });
    }

    #[test]
    fn ignored_paths_never_become_deletion_candidates() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let brought = TreeIndex::default();
            let e = entry(dir.path(), "node_modules/pkg/index.js", b"dep");
            let mut snap = Snapshot::default();
            snap.files
                .insert("node_modules/pkg/index.js".into(), e.hash.clone());
            snap.last_brought_file_count = 1;
            this_tree
                .files
                .insert("node_modules/pkg/index.js".into(), e);
            // Force zone via SyncRules defaults (node_modules ignored).
            let rules = SyncRules::load(dir.path());
            assert_eq!(rules.zone("node_modules/pkg/index.js"), PathZone::Ignored);
            let (rows, summary, _) = classify(&this_tree, &brought, &rules, &snap, "s1");
            assert_eq!(summary.deleted_brought, 0);
            assert!(rows.is_empty());
        });
    }

    #[test]
    fn keep_despite_absent_skips_deletion() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let mut this_tree = TreeIndex::default();
            let brought = TreeIndex::default();
            let e = entry(dir.path(), "local-only.txt", b"stay");
            let mut snap = Snapshot::default();
            snap.files.insert("local-only.txt".into(), e.hash.clone());
            snap.keep_despite_absent.insert("local-only.txt".into());
            snap.last_brought_file_count = 1;
            this_tree.files.insert("local-only.txt".into(), e);
            // Make brought "complete" with one unrelated file so incomplete guard doesn't fire
            // Actually brought is empty — set last_brought low so incomplete doesn't trigger
            snap.last_brought_file_count = 0;
            // With empty snapshot baseline size 1, threshold won't fire (expected < 20)
            let rules = SyncRules::load(dir.path());
            let (rows, summary, _) = classify(&this_tree, &brought, &rules, &snap, "s1");
            assert_eq!(summary.deleted_brought, 0);
            assert_eq!(rows[0].class, ChangeClass::Keep);
        });
    }
}
