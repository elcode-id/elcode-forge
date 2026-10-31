//! Atomic transaction journal and rollback manager for `eforge fix` (`.eforge/tx/<tx_id>/`).
//!
//! Ensures every surgical codemod applied to disk can be rolled back byte-for-byte on error
//! or on user command (`eforge fix --rollback`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::path_guard::resolve_safe_path;
use forge_core::{FixSuggestion, ForgeCache, ForgeError, TextEdit, EFORGE_DIR};
use serde::{Deserialize, Serialize};

/// Subdirectory inside `.eforge/` storing transaction journals and file backups.
pub const TX_DIR_NAME: &str = "tx";

/// Backup metadata for a single file touched during a fix transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionFileBackup {
    pub relative_path: String,
    pub existed_before: bool,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
}

/// Persistent transaction manifest stored at `.eforge/tx/<tx_id>/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_id: String,
    pub edits_applied: usize,
    pub files: Vec<TransactionFileBackup>,
}

/// Returns the root `.eforge/tx` directory path.
#[must_use]
pub fn tx_root_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(EFORGE_DIR).join(TX_DIR_NAME)
}

/// Lists all recorded transactions in `.eforge/tx/`, sorted deterministically by `tx_id` descending.
pub fn list_transactions(repo_root: &Path) -> Result<Vec<TransactionRecord>, ForgeError> {
    let root = tx_root_dir(repo_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&root).map_err(|source| ForgeError::Io {
        path: root.clone(),
        source,
    })?;

    let mut records = Vec::new();
    for entry in entries.flatten() {
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&manifest_path) {
                if let Ok(rec) = serde_json::from_str::<TransactionRecord>(&raw) {
                    records.push(rec);
                }
            }
        }
    }

    records.sort_by(|a, b| b.tx_id.cmp(&a.tx_id));
    Ok(records)
}

/// Applies all [`FixSuggestion`]s in `suggestions` atomically, backing up every affected file
/// into `.eforge/tx/<tx_id>/backups/` before modifying disk. If any file write fails,
/// automatically restores all previously touched files before returning the error.
pub fn apply_fix_plan_transactional(
    repo_root: &Path,
    suggestions: &[FixSuggestion],
) -> Result<TransactionRecord, ForgeError> {
    let all_edits: Vec<&TextEdit> = suggestions.iter().flat_map(|s| s.edits.iter()).collect();
    if all_edits.is_empty() {
        return Ok(TransactionRecord {
            tx_id: "tx-noop".to_string(),
            edits_applied: 0,
            files: Vec::new(),
        });
    }

    // Group edits by normalized relative file path
    let mut touched_paths = BTreeSet::new();
    for edit in &all_edits {
        touched_paths.insert(edit.file_path.replace('\\', "/"));
    }

    // Compute deterministic transaction ID from edit payload
    let serialized_edits =
        serde_json::to_vec(&all_edits).map_err(|e| ForgeError::Serialization {
            message: e.to_string(),
        })?;
    let digest = ForgeCache::hash_bytes(&serialized_edits);
    let short_hash = &digest[..12.min(digest.len())];
    let existing_count = list_transactions(repo_root).map(|v| v.len()).unwrap_or(0) + 1;
    let tx_id = format!("tx-{existing_count:04}-{short_hash}");

    let tx_dir = tx_root_dir(repo_root).join(&tx_id);
    let backups_dir = tx_dir.join("backups");
    fs::create_dir_all(&backups_dir).map_err(|source| ForgeError::Io {
        path: backups_dir.clone(),
        source,
    })?;

    let mut file_backups = Vec::new();

    // Step 1: Backup all target files before touching any file on disk
    for rel_path in &touched_paths {
        let target_full = resolve_possible_new_path(repo_root, rel_path)?;
        let existed = target_full.is_file();
        let before_hash = if existed {
            let bytes = fs::read(&target_full).map_err(|source| ForgeError::Io {
                path: target_full.clone(),
                source,
            })?;
            let backup_dest = backups_dir.join(rel_path);
            if let Some(parent) = backup_dest.parent() {
                fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::write(&backup_dest, &bytes).map_err(|source| ForgeError::Io {
                path: backup_dest,
                source,
            })?;
            Some(ForgeCache::hash_bytes(&bytes))
        } else {
            None
        };

        file_backups.push(TransactionFileBackup {
            relative_path: rel_path.clone(),
            existed_before: existed,
            before_hash,
            after_hash: None,
        });
    }

    // Step 2: Apply edits file-by-file; rollback on any error
    for backup in &mut file_backups {
        let file_edits: Vec<&&TextEdit> = all_edits
            .iter()
            .filter(|e| e.file_path.replace('\\', "/") == backup.relative_path)
            .collect();

        if let Err(err) = apply_edits_to_single_file(repo_root, &backup.relative_path, &file_edits)
        {
            let _ = restore_from_backups(repo_root, &backups_dir, &file_backups);
            return Err(err);
        }

        let target_full = repo_root.join(&backup.relative_path);
        if target_full.is_file() {
            if let Ok(bytes) = fs::read(&target_full) {
                backup.after_hash = Some(ForgeCache::hash_bytes(&bytes));
            }
        }
    }

    let record = TransactionRecord {
        tx_id,
        edits_applied: all_edits.len(),
        files: file_backups,
    };

    let manifest_path = tx_dir.join("manifest.json");
    let manifest_json =
        serde_json::to_string_pretty(&record).map_err(|e| ForgeError::Serialization {
            message: e.to_string(),
        })?;
    fs::write(&manifest_path, manifest_json).map_err(|source| ForgeError::Io {
        path: manifest_path,
        source,
    })?;

    Ok(record)
}

/// Rolls back the specified transaction `tx_id` (or the latest transaction if `None`),
/// restoring all modified files byte-for-byte and removing any newly created files.
pub fn rollback_transaction(
    repo_root: &Path,
    tx_id: Option<&str>,
) -> Result<TransactionRecord, ForgeError> {
    let transactions = list_transactions(repo_root)?;
    if transactions.is_empty() {
        return Err(ForgeError::Config {
            message: "no recorded fix transactions found in .eforge/tx/".to_string(),
        });
    }

    let record = if let Some(target_id) = tx_id {
        transactions
            .into_iter()
            .find(|t| t.tx_id == target_id)
            .ok_or_else(|| ForgeError::Config {
                message: format!("transaction '{target_id}' not found in .eforge/tx/"),
            })?
    } else {
        transactions
            .into_iter()
            .next()
            .ok_or_else(|| ForgeError::Config {
                message: "no transaction available to roll back".to_string(),
            })?
    };

    let tx_dir = tx_root_dir(repo_root).join(&record.tx_id);
    let backups_dir = tx_dir.join("backups");
    restore_from_backups(repo_root, &backups_dir, &record.files)?;

    // Remove the rolled-back transaction directory so subsequent rollbacks step back cleanly
    let _ = fs::remove_dir_all(&tx_dir);

    Ok(record)
}

fn resolve_possible_new_path(repo_root: &Path, rel_path: &str) -> Result<PathBuf, ForgeError> {
    if rel_path.contains("..") || Path::new(rel_path).is_absolute() {
        return Err(ForgeError::PathTraversal {
            repo_root: repo_root.to_path_buf(),
            candidate: PathBuf::from(rel_path),
        });
    }
    let candidate = Path::new(rel_path);
    if repo_root.join(candidate).exists() {
        resolve_safe_path(repo_root, candidate)
    } else {
        Ok(repo_root.join(candidate))
    }
}

fn apply_edits_to_single_file(
    repo_root: &Path,
    rel_path: &str,
    edits: &[&&TextEdit],
) -> Result<(), ForgeError> {
    let full_path = resolve_possible_new_path(repo_root, rel_path)?;

    if !full_path.exists() {
        // Creating a brand-new companion file
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let combined: String = edits
            .iter()
            .map(|e| e.replacement_text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&full_path, combined).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;
        return Ok(());
    }

    let original_content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
        path: full_path.clone(),
        source,
    })?;
    let uses_crlf = original_content.contains("\r\n");
    let line_ending = if uses_crlf { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = original_content.lines().map(String::from).collect();

    // Sort edits bottom-up by start_line descending so line indices remain valid
    let mut sorted_edits: Vec<&&TextEdit> = edits.to_vec();
    sorted_edits.sort_by_key(|a| std::cmp::Reverse(a.start_line));

    for edit in sorted_edits {
        let start_idx = edit.start_line.saturating_sub(1) as usize;
        let end_idx = (edit.end_line as usize).min(lines.len());
        if start_idx <= end_idx && start_idx <= lines.len() {
            let replacement_lines: Vec<String> = if edit.replacement_text.is_empty() {
                Vec::new()
            } else {
                edit.replacement_text.lines().map(String::from).collect()
            };
            lines.splice(start_idx..end_idx, replacement_lines);
        }
    }

    // Clean up trailing comma before `}` in JSON files when removing a dependency line
    if rel_path.ends_with(".json") {
        sanitize_json_trailing_commas(&mut lines);
    }

    let mut updated = lines.join(line_ending);
    if original_content.ends_with('\n') && !updated.ends_with('\n') {
        updated.push_str(line_ending);
    }

    fs::write(&full_path, updated).map_err(|source| ForgeError::Io {
        path: full_path,
        source,
    })?;
    Ok(())
}

fn sanitize_json_trailing_commas(lines: &mut [String]) {
    for i in 0..lines.len().saturating_sub(1) {
        let next_trimmed = lines[i + 1].trim();
        if (next_trimmed.starts_with('}') || next_trimmed.starts_with(']'))
            && lines[i].trim_end().ends_with(',')
        {
            if let Some(comma_pos) = lines[i].rfind(',') {
                lines[i].remove(comma_pos);
            }
        }
    }
}

fn restore_from_backups(
    repo_root: &Path,
    backups_dir: &Path,
    files: &[TransactionFileBackup],
) -> Result<(), ForgeError> {
    for file in files {
        let target_full = repo_root.join(&file.relative_path);
        if file.existed_before {
            let backup_src = backups_dir.join(&file.relative_path);
            if backup_src.is_file() {
                let bytes = fs::read(&backup_src).map_err(|source| ForgeError::Io {
                    path: backup_src,
                    source,
                })?;
                fs::write(&target_full, bytes).map_err(|source| ForgeError::Io {
                    path: target_full,
                    source,
                })?;
            }
        } else if target_full.is_file() {
            let _ = fs::remove_file(&target_full);
        }
    }
    Ok(())
}
