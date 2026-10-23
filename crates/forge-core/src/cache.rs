use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;
use crate::ir::FileNode;

/// Local directory name for ELCODE Forge incremental cache and index (`.eforge/`).
pub const EFORGE_DIR: &str = ".eforge";

/// Persisted metadata stored at `.eforge/metadata.json` for cache invalidation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub analyzer_version: String,
    pub config_hash: String,
    pub file_hashes: BTreeMap<String, String>,
}

/// Content-addressed remote cache adapter trait.
pub trait RemoteCacheAdapter: Send + Sync {
    /// Retrieves object bytes for a namespace and cache key from remote storage.
    fn get_bytes(&self, namespace: &str, cache_key: &str) -> Result<Option<Vec<u8>>, ForgeError>;
    /// Stores object bytes for a namespace and cache key into remote storage.
    fn put_bytes(&self, namespace: &str, cache_key: &str, data: &[u8]) -> Result<(), ForgeError>;
}

/// In-memory cache adapter ideal for testing, benchmarks, and single-process sharing.
#[derive(Debug, Default)]
pub struct InMemoryRemoteCacheAdapter {
    storage: std::sync::RwLock<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryRemoteCacheAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RemoteCacheAdapter for InMemoryRemoteCacheAdapter {
    fn get_bytes(&self, namespace: &str, cache_key: &str) -> Result<Option<Vec<u8>>, ForgeError> {
        let key = format!("{namespace}/{cache_key}");
        let guard = self.storage.read().map_err(|_| ForgeError::Config {
            message: "in-memory cache lock poisoned".to_string(),
        })?;
        Ok(guard.get(&key).cloned())
    }

    fn put_bytes(&self, namespace: &str, cache_key: &str, data: &[u8]) -> Result<(), ForgeError> {
        let key = format!("{namespace}/{cache_key}");
        let mut guard = self.storage.write().map_err(|_| ForgeError::Config {
            message: "in-memory cache lock poisoned".to_string(),
        })?;
        guard.insert(key, data.to_vec());
        Ok(())
    }
}

/// Filesystem-backed remote cache adapter for CI runners sharing persistent directory or mounted network volume.
#[derive(Debug, Clone)]
pub struct FileRemoteCacheAdapter {
    base_dir: PathBuf,
}

impl FileRemoteCacheAdapter {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl RemoteCacheAdapter for FileRemoteCacheAdapter {
    fn get_bytes(&self, namespace: &str, cache_key: &str) -> Result<Option<Vec<u8>>, ForgeError> {
        let file_path = self
            .base_dir
            .join(namespace)
            .join(format!("{cache_key}.json"));
        if !file_path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&file_path).map_err(|source| ForgeError::Io {
            path: file_path,
            source,
        })?;
        Ok(Some(bytes))
    }

    fn put_bytes(&self, namespace: &str, cache_key: &str, data: &[u8]) -> Result<(), ForgeError> {
        let file_path = self
            .base_dir
            .join(namespace)
            .join(format!("{cache_key}.json"));
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        fs::write(&file_path, data).map_err(|source| ForgeError::Io {
            path: file_path,
            source,
        })
    }
}

/// HTTP remote cache adapter connecting to ELCODE Forge Server's object cache endpoint.
#[derive(Debug, Clone)]
pub struct HttpRemoteCacheAdapter {
    server_url: String,
    token: Option<String>,
}

impl HttpRemoteCacheAdapter {
    pub fn new(server_url: String, token: Option<String>) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            token,
        }
    }
}

impl RemoteCacheAdapter for HttpRemoteCacheAdapter {
    fn get_bytes(&self, namespace: &str, cache_key: &str) -> Result<Option<Vec<u8>>, ForgeError> {
        let url = format!("{}/api/v1/cache/{namespace}/{cache_key}", self.server_url);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        let mut req = agent.get(&url);
        if let Some(ref tok) = self.token {
            req = req.set("Authorization", &format!("Bearer {tok}"));
        }

        match req.call() {
            Ok(resp) => {
                let mut reader = resp.into_reader();
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|source| {
                    ForgeError::Io {
                        path: PathBuf::from(&url),
                        source,
                    }
                })?;
                Ok(Some(buf))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(err) => Err(ForgeError::Sync {
                message: format!("failed to fetch remote cache from {url}: {err}"),
            }),
        }
    }

    fn put_bytes(&self, namespace: &str, cache_key: &str, data: &[u8]) -> Result<(), ForgeError> {
        let url = format!("{}/api/v1/cache/{namespace}/{cache_key}", self.server_url);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(15))
            .build();

        let mut req = agent.put(&url).set("Content-Type", "application/json");
        if let Some(ref tok) = self.token {
            req = req.set("Authorization", &format!("Bearer {tok}"));
        }

        match req.send_bytes(data) {
            Ok(_) => Ok(()),
            Err(err) => Err(ForgeError::Sync {
                message: format!("failed to upload remote cache to {url}: {err}"),
            }),
        }
    }
}

/// Computes a deterministic content-addressed cache key representing the analysis inputs:
/// - namespace domain (e.g. "architecture_graph", "dependency_report")
/// - configuration digest
/// - forge engine version
/// - sorted relative paths and BLAKE3 content hashes of relevant input files
pub fn compute_analysis_cache_key(
    namespace: &str,
    config_hash: &str,
    files: &[FileNode],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"FORGE_CACHE_KEY_V1\n");
    hasher.update(namespace.as_bytes());
    hasher.update(b"\n");
    hasher.update(config_hash.as_bytes());
    hasher.update(b"\n");
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(b"\n");

    let mut sorted: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.relative_path.as_str(), f.content_hash.as_str()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (p, h) in sorted {
        hasher.update(p.as_bytes());
        hasher.update(b":");
        hasher.update(h.as_bytes());
        hasher.update(b"\n");
    }

    hasher.finalize().to_hex().to_string()
}

/// Manages the `.eforge/` directory layout (`.eforge/cache/`, `.eforge/index/`,
/// `.eforge/metadata.json`) and deterministic `blake3` file content hashing.
#[derive(Debug, Clone)]
pub struct ForgeCache {
    root_dir: PathBuf,
}

impl ForgeCache {
    /// Initializes `.eforge/`, `.eforge/cache/`, and `.eforge/index/` inside `repo_root`.
    pub fn init(repo_root: &Path) -> Result<Self, ForgeError> {
        let root_dir = repo_root.join(EFORGE_DIR);
        let cache_dir = root_dir.join("cache");
        let index_dir = root_dir.join("index");

        fs::create_dir_all(&cache_dir).map_err(|source| ForgeError::Io {
            path: cache_dir,
            source,
        })?;
        fs::create_dir_all(&index_dir).map_err(|source| ForgeError::Io {
            path: index_dir,
            source,
        })?;

        Ok(Self { root_dir })
    }

    /// Returns the `.eforge` root directory path.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns the `.eforge/cache` subdirectory path.
    pub fn cache_dir(&self) -> PathBuf {
        self.root_dir.join("cache")
    }

    /// Returns the `.eforge/index` subdirectory path.
    pub fn index_dir(&self) -> PathBuf {
        self.root_dir.join("index")
    }

    /// Returns the `.eforge/metadata.json` file path.
    pub fn metadata_path(&self) -> PathBuf {
        self.root_dir.join("metadata.json")
    }

    /// Computes path for a cached object: `.eforge/cache/<namespace>/<cache_key>.json`.
    pub fn object_path(&self, namespace: &str, cache_key: &str) -> PathBuf {
        self.cache_dir()
            .join(namespace)
            .join(format!("{cache_key}.json"))
    }

    /// Computes a deterministic 64-character lowercase hexadecimal BLAKE3 digest of `bytes`.
    pub fn hash_bytes(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    /// Loads `.eforge/metadata.json` if it exists, or returns `Ok(None)` if not yet created.
    pub fn load_metadata(&self) -> Result<Option<CacheMetadata>, ForgeError> {
        let path = self.metadata_path();
        if !path.is_file() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|source| ForgeError::Io {
            path: path.clone(),
            source,
        })?;

        let meta = serde_json::from_str::<CacheMetadata>(&content).map_err(|err| {
            ForgeError::Serialization {
                message: format!("failed to parse '{}': {err}", path.display()),
            }
        })?;

        Ok(Some(meta))
    }

    /// Saves [`CacheMetadata`] deterministically to `.eforge/metadata.json`.
    pub fn save_metadata(&self, meta: &CacheMetadata) -> Result<(), ForgeError> {
        let path = self.metadata_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let serialized =
            serde_json::to_string_pretty(meta).map_err(|err| ForgeError::Serialization {
                message: format!("failed to serialize cache metadata: {err}"),
            })?;

        fs::write(&path, serialized).map_err(|source| ForgeError::Io { path, source })
    }

    /// Computes the sorted list of relative file paths that were added, modified, or deleted
    /// compared to `.eforge/metadata.json`.
    ///
    /// If no previous metadata exists, or if `config_hash` or `analyzer_version` changed,
    /// all files in `current` (plus any previously tracked files) are marked as changed.
    pub fn diff_changed_files(
        &self,
        current: &BTreeMap<String, String>,
        config_hash: &str,
    ) -> Vec<String> {
        let Ok(Some(prev)) = self.load_metadata() else {
            return current.keys().cloned().collect();
        };

        if prev.analyzer_version != env!("CARGO_PKG_VERSION") || prev.config_hash != config_hash {
            let mut all_paths: BTreeSet<String> = current.keys().cloned().collect();
            all_paths.extend(prev.file_hashes.keys().cloned());
            return all_paths.into_iter().collect();
        }

        let mut changed = BTreeSet::new();

        for (path, hash) in current {
            match prev.file_hashes.get(path) {
                Some(old_hash) if old_hash == hash => {}
                _ => {
                    changed.insert(path.clone());
                }
            }
        }

        for old_path in prev.file_hashes.keys() {
            if !current.contains_key(old_path) {
                changed.insert(old_path.clone());
            }
        }

        changed.into_iter().collect()
    }

    /// Retrieves an object from local cache if present.
    pub fn get_cached<T: serde::de::DeserializeOwned>(
        &self,
        namespace: &str,
        cache_key: &str,
    ) -> Result<Option<T>, ForgeError> {
        let path = self.object_path(namespace, cache_key);
        if !path.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|source| ForgeError::Io {
            path: path.clone(),
            source,
        })?;
        let item = serde_json::from_str::<T>(&text).map_err(|err| ForgeError::Serialization {
            message: format!(
                "failed to parse cached object from '{}': {err}",
                path.display()
            ),
        })?;
        Ok(Some(item))
    }

    /// Stores an object into local cache.
    pub fn put_cached<T: serde::Serialize>(
        &self,
        namespace: &str,
        cache_key: &str,
        data: &T,
    ) -> Result<(), ForgeError> {
        let path = self.object_path(namespace, cache_key);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let text = serde_json::to_string(data).map_err(|err| ForgeError::Serialization {
            message: format!("failed to serialize cache object: {err}"),
        })?;
        fs::write(&path, text).map_err(|source| ForgeError::Io { path, source })
    }

    /// Retrieves an object from local disk cache, falling back to optional remote cache adapter.
    /// If retrieved from remote, populates the local disk cache.
    pub fn get_with_remote<T: serde::de::DeserializeOwned + serde::Serialize>(
        &self,
        remote: Option<&dyn RemoteCacheAdapter>,
        namespace: &str,
        cache_key: &str,
    ) -> Result<Option<T>, ForgeError> {
        if let Some(local_item) = self.get_cached::<T>(namespace, cache_key)? {
            return Ok(Some(local_item));
        }

        if let Some(remote_adapter) = remote {
            if let Ok(Some(bytes)) = remote_adapter.get_bytes(namespace, cache_key) {
                if let Ok(item) = serde_json::from_slice::<T>(&bytes) {
                    let _ = self.put_cached(namespace, cache_key, &item);
                    return Ok(Some(item));
                }
            }
        }

        Ok(None)
    }

    /// Saves an object to local disk cache and optional remote cache adapter.
    pub fn put_with_remote<T: serde::Serialize>(
        &self,
        remote: Option<&dyn RemoteCacheAdapter>,
        namespace: &str,
        cache_key: &str,
        data: &T,
    ) -> Result<(), ForgeError> {
        self.put_cached(namespace, cache_key, data)?;

        if let Some(remote_adapter) = remote {
            if let Ok(bytes) = serde_json::to_vec(data) {
                let _ = remote_adapter.put_bytes(namespace, cache_key, &bytes);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("eforge_cache_test_{label}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn initializes_eforge_directory_structure_and_hashes_bytes() {
        let repo = unique_temp_dir("init");
        let cache = ForgeCache::init(&repo);
        assert!(cache.is_ok());

        if let Ok(c) = cache {
            assert!(c.root_dir().ends_with(".eforge"));
            assert!(c.cache_dir().is_dir());
            assert!(c.index_dir().is_dir());

            let h1 = ForgeCache::hash_bytes(b"export const x = 42;");
            let h2 = ForgeCache::hash_bytes(b"export const x = 42;");
            let h3 = ForgeCache::hash_bytes(b"export const x = 43;");
            assert_eq!(h1.len(), 64);
            assert_eq!(h1, h2);
            assert_ne!(h1, h3);
        }

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn saves_loads_and_diffs_changed_files_deterministically() {
        let repo = unique_temp_dir("diff");
        let cache = ForgeCache::init(&repo);
        assert!(cache.is_ok());

        if let Ok(c) = cache {
            let mut initial_files = BTreeMap::new();
            initial_files.insert(
                "src/a.ts".to_string(),
                ForgeCache::hash_bytes(b"const a = 1;"),
            );
            initial_files.insert(
                "src/b.ts".to_string(),
                ForgeCache::hash_bytes(b"const b = 1;"),
            );

            // Cold cache -> all files changed
            let cold_diff = c.diff_changed_files(&initial_files, "cfg_v1");
            assert_eq!(cold_diff, vec!["src/a.ts", "src/b.ts"]);

            let meta = CacheMetadata {
                analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
                config_hash: "cfg_v1".to_string(),
                file_hashes: initial_files.clone(),
            };
            assert!(c.save_metadata(&meta).is_ok());

            // Warm cache with identical hashes -> 0 changed files
            let warm_diff = c.diff_changed_files(&initial_files, "cfg_v1");
            assert!(warm_diff.is_empty());

            // Modify src/b.ts and add src/c.ts, remove src/a.ts
            let mut next_files = BTreeMap::new();
            next_files.insert(
                "src/b.ts".to_string(),
                ForgeCache::hash_bytes(b"const b = 2;"),
            );
            next_files.insert(
                "src/c.ts".to_string(),
                ForgeCache::hash_bytes(b"const c = 1;"),
            );

            let incremental_diff = c.diff_changed_files(&next_files, "cfg_v1");
            assert_eq!(incremental_diff, vec!["src/a.ts", "src/b.ts", "src/c.ts"]);

            // Config hash change invalidates all files
            let invalidated_diff = c.diff_changed_files(&initial_files, "cfg_v2");
            assert_eq!(invalidated_diff, vec!["src/a.ts", "src/b.ts"]);
        }

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_remote_cache_adapters_and_object_storage() {
        let repo = unique_temp_dir("remote");
        let cache = ForgeCache::init(&repo);
        assert!(cache.is_ok());

        if let Ok(c) = cache {
            let adapter = InMemoryRemoteCacheAdapter::new();
            let key = "test_object_key_123";
            let val = vec!["module_a".to_string(), "module_b".to_string()];

            // 1. Put with remote (saves to local cache and in-memory remote)
            let put_res = c.put_with_remote(Some(&adapter), "modules", key, &val);
            assert!(put_res.is_ok());

            // 2. Local hit
            let local_get = c.get_cached::<Vec<String>>("modules", key);
            assert!(local_get.is_ok());
            if let Ok(Some(loaded)) = local_get {
                assert_eq!(loaded, val);
            }

            // 3. Clear local file to test fallback to remote adapter
            let local_path = c.object_path("modules", key);
            let _ = fs::remove_file(&local_path);
            assert!(!local_path.exists());

            // 4. Remote hit repopulates local cache
            let remote_get = c.get_with_remote::<Vec<String>>(Some(&adapter), "modules", key);
            assert!(remote_get.is_ok());
            if let Ok(Some(loaded)) = remote_get {
                assert_eq!(loaded, val);
                assert!(local_path.exists());
            }
        }

        let _ = fs::remove_dir_all(&repo);
    }
}
