use crate::engine::types::*;
use crate::engine::SyncRules;
use crate::engine::Snapshot;
use std::collections::HashMap;
use uuid::Uuid;

pub fn classify(
    this_tree: &TreeIndex,
    brought_tree: &TreeIndex,
    rules: &SyncRules,
    snapshot: &Snapshot,
    session_id: &str,
) -> (Vec<FileRow>, ScanSummary) {
    let mut summary = ScanSummary {
        total_this: this_tree.files.len() as u64,
        total_brought: brought_tree.files.len() as u64,
        errors: (this_tree.errors.len() + brought_tree.errors.len()) as u64,
        ..Default::default()
    };

    let mut rows: Vec<FileRow> = Vec::new();
    let mut used_brought: HashMap<String, bool> = HashMap::new();

    // Detect moves: deleted on this + added on brought with same hash (or vice versa)
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
                // hide identical
            }
            (Some(t), Some(b)) => {
                // both exist, different
                let class = if let Some(s) = snap {
                    if s == &t.hash && s != &b.hash {
                        ChangeClass::BringIn
                    } else if s == &b.hash && s != &t.hash {
                        ChangeClass::Keep
                    } else if s != &t.hash && s != &b.hash {
                        ChangeClass::NeedsYou
                    } else {
                        ChangeClass::NeedsYou
                    }
                } else {
                    // first sync: both differ → needs you
                    ChangeClass::NeedsYou
                };

                let decision = match class {
                    ChangeClass::BringIn => Decision::BringIn,
                    ChangeClass::Keep => Decision::Keep,
                    _ => Decision::Keep, // safe default until user picks
                };

                match class {
                    ChangeClass::BringIn => summary.bring_in += 1,
                    ChangeClass::Keep => summary.keep += 1,
                    ChangeClass::NeedsYou => summary.needs_you += 1,
                    _ => {}
                }

                rows.push(row(
                    session_id,
                    &rel,
                    &group,
                    class,
                    decision,
                    zone,
                    Some(t),
                    Some(b),
                    None,
                ));
            }
            (None, Some(b)) => {
                // added on brought side — check move
                if let Some(old_paths) = this_by_hash.get(b.hash.as_str()) {
                    if let Some(old) = old_paths.iter().find(|p| !brought_tree.files.contains_key(**p))
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
                        used_brought.insert(rel.clone(), true);
                        continue;
                    }
                }
                let class = if snap.is_none() || snap.map(|s| s != &b.hash).unwrap_or(true) {
                    ChangeClass::BringIn
                } else {
                    ChangeClass::BringIn
                };
                summary.bring_in += 1;
                rows.push(row(
                    session_id,
                    &rel,
                    &group,
                    class,
                    Decision::BringIn,
                    zone,
                    None,
                    Some(b),
                    None,
                ));
            }
            (Some(t), None) => {
                // missing on brought — deletion or move
                if let Some(new_paths) = brought_by_hash.get(t.hash.as_str()) {
                    if new_paths.iter().any(|p| !this_tree.files.contains_key(*p)) {
                        // move handled from the added side
                        continue;
                    }
                }
                if snap.map(|s| s == &t.hash).unwrap_or(false) {
                    // was synced, now gone on brought → offer delete, default skip
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
                } else {
                    // local-only file
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

    let _ = used_brought;
    let _ = session_id;
    rows.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    (rows, summary)
}

fn invert_hash_index(files: &HashMap<String, FileEntry>) -> HashMap<&str, Vec<&str>> {
    let mut map: HashMap<&str, Vec<&str>> = HashMap::new();
    for (path, entry) in files {
        map.entry(entry.hash.as_str()).or_default().push(path.as_str());
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
        let mut this_tree = TreeIndex::default();
        let mut brought = TreeIndex::default();
        this_tree
            .files
            .insert("a.txt".into(), entry(dir.path(), "a-this.txt", b"local"));
        // fix rel
        let e = this_tree.files.get_mut("a.txt").unwrap();
        e.rel_path = "a.txt".into();
        brought
            .files
            .insert("a.txt".into(), entry(dir.path(), "a-brought.txt", b"remote"));
        let e = brought.files.get_mut("a.txt").unwrap();
        e.rel_path = "a.txt".into();

        let rules = SyncRules::load(dir.path());
        let snap = Snapshot::default();
        let (rows, summary) = classify(&this_tree, &brought, &rules, &snap, "s1");
        assert_eq!(summary.needs_you, 1);
        assert_eq!(rows[0].class, ChangeClass::NeedsYou);
    }

    #[test]
    fn snapshot_makes_bring_in_automatic() {
        let dir = tempdir().unwrap();
        let mut this_tree = TreeIndex::default();
        let mut brought = TreeIndex::default();
        let local = entry(dir.path(), "l.txt", b"same-base");
        let remote = entry(dir.path(), "r.txt", b"new-on-internet");
        this_tree.files.insert("svc/a.txt".into(), FileEntry {
            rel_path: "svc/a.txt".into(),
            size: local.size,
            hash: local.hash.clone(),
            kind: FileKind::Text,
        });
        brought.files.insert("svc/a.txt".into(), FileEntry {
            rel_path: "svc/a.txt".into(),
            size: remote.size,
            hash: remote.hash.clone(),
            kind: FileKind::Text,
        });
        let mut snap = Snapshot::default();
        snap.files.insert("svc/a.txt".into(), local.hash);
        let rules = SyncRules::load(dir.path());
        let (rows, summary) = classify(&this_tree, &brought, &rules, &snap, "s1");
        assert_eq!(summary.bring_in, 1);
        assert_eq!(rows[0].class, ChangeClass::BringIn);
        assert_eq!(rows[0].decision, Decision::BringIn);
    }
}
