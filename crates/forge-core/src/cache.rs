use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

/// Local directory name for ELCODE Forge incremental cache and index (`.eforge/`).
pub const EFORGE_DIR: &str = ".eforge";

/// Persisted metadata stored at `.eforge/metadata.json` for cache invalidation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub analyzer_version: String,
    pub config_hash: String,
    pub file_hashes: BTreeMap<String, String>,
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
}
