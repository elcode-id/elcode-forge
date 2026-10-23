use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use forge_analyzer::{
    analyze_polyglot_dependencies_cached, build_polyglot_architecture_graph_cached, scan_repository,
};
use forge_core::cache::{ForgeCache, InMemoryRemoteCacheAdapter};

#[test]
fn test_cached_analysis_runtime_and_invalidation() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .and_then(|p| p.parent().map(|p2| p2.to_path_buf()))
        .map(|p| p.join("fixtures").join("next-app"))
        .unwrap_or_else(|| PathBuf::from("fixtures/next-app"));

    let discovery_res = scan_repository(&repo_root, &[]);
    assert!(discovery_res.is_ok());
    let discovery = match discovery_res {
        Ok(d) => d,
        Err(_) => return,
    };
    assert!(!discovery.files.is_empty());

    let temp_cache_dir =
        std::env::temp_dir().join(format!("eforge_analyzer_cache_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_cache_dir);
    let cache_res = ForgeCache::init(&temp_cache_dir);
    assert!(cache_res.is_ok());
    let cache = match cache_res {
        Ok(c) => c,
        Err(_) => return,
    };

    let remote_adapter = InMemoryRemoteCacheAdapter::new();
    let config_hash = "test_config_hash_v1";

    // 1. Cold Run
    let start_cold = Instant::now();
    let graph_cold_res = build_polyglot_architecture_graph_cached(
        &repo_root,
        &discovery.files,
        Some(&cache),
        Some(&remote_adapter),
        config_hash,
    );
    assert!(graph_cold_res.is_ok());
    let graph_cold = match graph_cold_res {
        Ok(g) => g,
        Err(_) => return,
    };

    let dep_cold_res = analyze_polyglot_dependencies_cached(
        &repo_root,
        &discovery.files,
        &graph_cold.external_imports,
        Some(&cache),
        Some(&remote_adapter),
        config_hash,
    );
    assert!(dep_cold_res.is_ok());
    let dep_cold = match dep_cold_res {
        Ok(d) => d,
        Err(_) => return,
    };
    let cold_duration = start_cold.elapsed();

    // 2. Warm Run (Cache Hit)
    let start_warm = Instant::now();
    let graph_warm_res = build_polyglot_architecture_graph_cached(
        &repo_root,
        &discovery.files,
        Some(&cache),
        Some(&remote_adapter),
        config_hash,
    );
    assert!(graph_warm_res.is_ok());
    let graph_warm = match graph_warm_res {
        Ok(g) => g,
        Err(_) => return,
    };

    let dep_warm_res = analyze_polyglot_dependencies_cached(
        &repo_root,
        &discovery.files,
        &graph_warm.external_imports,
        Some(&cache),
        Some(&remote_adapter),
        config_hash,
    );
    assert!(dep_warm_res.is_ok());
    let dep_warm = match dep_warm_res {
        Ok(d) => d,
        Err(_) => return,
    };
    let warm_duration = start_warm.elapsed();

    assert_eq!(graph_cold, graph_warm);
    assert_eq!(dep_cold, dep_warm);

    println!(
        "Cold duration: {:?}, Warm duration: {:?}",
        cold_duration, warm_duration
    );
    assert!(
        warm_duration.as_millis() < 500,
        "warm analysis took {:?} (expected < 500ms)",
        warm_duration
    );

    // 3. Invalidation: change config_hash -> cache miss and recomputed
    let graph_invalidated_res = build_polyglot_architecture_graph_cached(
        &repo_root,
        &discovery.files,
        Some(&cache),
        Some(&remote_adapter),
        "test_config_hash_v2",
    );
    assert!(graph_invalidated_res.is_ok());
    if let Ok(graph_invalidated) = graph_invalidated_res {
        assert_eq!(graph_cold.modules.len(), graph_invalidated.modules.len());
    }

    let _ = fs::remove_dir_all(&temp_cache_dir);
}
