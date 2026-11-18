//! Synthetic tree timing — reports measured numbers; does not claim 500k/1M readiness.

use codesetu_lib::engine::rules::SyncRules;
use codesetu_lib::engine::walk::{scan_tree, ScanProgress};
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn synth_scan_1k_and_5k_files() {
    let dir = tempdir().unwrap();
    codesetu_lib::engine::paths::with_test_data_dir(dir.path(), || {
        for n in [1_000usize, 5_000usize] {
            let root = dir.path().join(format!("tree-{n}"));
            fs::create_dir_all(root.join("src")).unwrap();
            for i in 0..n {
                let sub = root.join("src").join(format!("b{}", i % 50));
                fs::create_dir_all(&sub).unwrap();
                fs::write(sub.join(format!("f{i}.txt")), format!("body-{i}")).unwrap();
            }
            // sprinkle ignored noise
            fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
            fs::write(root.join("node_modules/pkg/x.js"), "x").unwrap();

            let rules = SyncRules::load(&root);
            let progress = ScanProgress::new(Arc::new(AtomicBool::new(false)));
            let t0 = Instant::now();
            let idx = scan_tree(&root, &rules, &progress);
            let ms = t0.elapsed().as_millis();
            assert_eq!(idx.files.len(), n);
            eprintln!("synth_scan files={n} elapsed_ms={ms} cache_note=cold");

            // warm cache
            let progress2 = ScanProgress::new(Arc::new(AtomicBool::new(false)));
            let t1 = Instant::now();
            let idx2 = scan_tree(&root, &rules, &progress2);
            let warm_ms = t1.elapsed().as_millis();
            assert_eq!(idx2.files.len(), n);
            eprintln!(
                "synth_scan files={n} elapsed_ms={warm_ms} cache_note=warm hits≈{}",
                progress2
                    .files_cache_hits
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
        }
    });
}
