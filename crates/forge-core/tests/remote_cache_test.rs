use std::fs;

use forge_core::cache::{
    compute_analysis_cache_key, FileRemoteCacheAdapter, ForgeCache, InMemoryRemoteCacheAdapter,
    RemoteCacheAdapter,
};
use forge_core::ir::FileNode;

fn make_test_node(rel_path: &str, hash: &str) -> FileNode {
    FileNode {
        relative_path: rel_path.to_string(),
        language: Some("typescript".to_string()),
        size_bytes: 120,
        line_count: 10,
        content_hash: hash.to_string(),
        is_test: false,
    }
}

#[test]
fn test_compute_analysis_cache_key_determinism_and_order_independence() {
    let f1 = make_test_node("src/a.ts", "hash_aaa");
    let f2 = make_test_node("src/b.ts", "hash_bbb");

    let key1 = compute_analysis_cache_key("ast", "cfg_hash_1", &[f1.clone(), f2.clone()]);
    let key2 = compute_analysis_cache_key("ast", "cfg_hash_1", &[f2.clone(), f1.clone()]);

    assert_eq!(key1, key2);
    assert_eq!(key1.len(), 64);

    // Invalidation on config change
    let key_cfg_changed =
        compute_analysis_cache_key("ast", "cfg_hash_2", &[f1.clone(), f2.clone()]);
    assert_ne!(key1, key_cfg_changed);

    // Invalidation on file content change
    let f1_mod = make_test_node("src/a.ts", "hash_aaa_modified");
    let key_file_changed = compute_analysis_cache_key("ast", "cfg_hash_1", &[f1_mod, f2]);
    assert_ne!(key1, key_file_changed);
}

#[test]
fn test_remote_cache_adapters_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("remote_cache_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let local_cache = ForgeCache::init(&temp_dir);
    assert!(local_cache.is_ok());

    if let Ok(c) = local_cache {
        // 1. Test In-Memory Adapter
        let mem_adapter = InMemoryRemoteCacheAdapter::new();
        let key = "cache_obj_key_999";
        let data = vec!["module_alpha".to_string(), "module_beta".to_string()];

        let put_mem = c.put_with_remote(Some(&mem_adapter), "test_ns", key, &data);
        assert!(put_mem.is_ok());

        let mem_bytes = mem_adapter.get_bytes("test_ns", key);
        assert!(mem_bytes.is_ok());
        assert!(mem_bytes.unwrap_or(None).is_some());

        // 2. Test File Remote Adapter
        let remote_shared_dir = temp_dir.join("shared_remote_storage");
        let file_adapter = FileRemoteCacheAdapter::new(remote_shared_dir.clone());

        let put_file = c.put_with_remote(Some(&file_adapter), "test_ns", key, &data);
        assert!(put_file.is_ok());

        let file_bytes = file_adapter.get_bytes("test_ns", key);
        assert!(file_bytes.is_ok());
        assert!(file_bytes.unwrap_or(None).is_some());

        // 3. Clear local cache file and restore from file remote adapter
        let local_file = c.object_path("test_ns", key);
        let _ = fs::remove_file(&local_file);
        assert!(!local_file.exists());

        let fetched = c.get_with_remote::<Vec<String>>(Some(&file_adapter), "test_ns", key);
        assert!(fetched.is_ok());
        if let Ok(Some(loaded)) = fetched {
            assert_eq!(loaded, data);
            assert!(local_file.exists());
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
