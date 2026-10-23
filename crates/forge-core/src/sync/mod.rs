//! Content-addressed remote synchronization protocol, credential management,
//! and differential delta computation for ELCODE Forge (`eforge`).

pub mod credentials;
pub mod protocol;

pub use credentials::{
    default_credentials_path, home_dir, load_credentials, load_credentials_from_path,
    resolve_token, save_credentials, save_credentials_to_path, CredentialsConfig, ServerCredential,
};
pub use protocol::{DeltaSummary, SyncManifest, SyncPullResponse, SyncPushPayload, SyncStatus};

use crate::ir::{FileNode, Finding};

/// Computes a deterministic, content-addressed BLAKE3 hash representing the exact
/// analyzed state of repository files and findings.
///
/// Files and findings are canonicalized and sorted prior to hashing to guarantee
/// byte-for-byte reproducibility regardless of collection or traversal order.
pub fn compute_sync_content_hash(files: &[FileNode], findings: &[Finding]) -> String {
    let mut hasher = blake3::Hasher::new();

    // 1. Sort files canonically by relative path
    let mut sorted_files: Vec<&FileNode> = files.iter().collect();
    sorted_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    hasher.update(b"FORGE_SYNC_FILES_V1\n");
    for file in sorted_files {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b":");
        hasher.update(&file.size_bytes.to_le_bytes());
        hasher.update(b":");
        hasher.update(&file.line_count.to_le_bytes());
        hasher.update(b":");
        hasher.update(file.content_hash.as_bytes());
        hasher.update(b":");
        hasher.update(if file.is_test { b"1" } else { b"0" });
        hasher.update(b":");
        if let Some(ref lang) = file.language {
            hasher.update(lang.as_bytes());
        }
        hasher.update(b"\n");
    }

    // 2. Sort findings canonically by finding ID
    let mut sorted_findings: Vec<&Finding> = findings.iter().collect();
    sorted_findings.sort_by(|a, b| a.id.cmp(&b.id));

    hasher.update(b"FORGE_SYNC_FINDINGS_V1\n");
    for finding in sorted_findings {
        hasher.update(finding.id.as_bytes());
        hasher.update(b":");
        hasher.update(finding.rule_id.as_bytes());
        hasher.update(b":");
        hasher.update(finding.category.as_bytes());
        hasher.update(b":");
        hasher.update(format!("{:?}", finding.severity).as_bytes());
        hasher.update(b":");
        hasher.update(format!("{:?}", finding.confidence).as_bytes());
        hasher.update(b":");
        if let Some(ref loc) = finding.location {
            hasher.update(loc.file_path.as_bytes());
            hasher.update(b"#");
            if let Some(start) = loc.start_line {
                hasher.update(&start.to_le_bytes());
            }
            hasher.update(b"-");
            if let Some(end) = loc.end_line {
                hasher.update(&end.to_le_bytes());
            }
        }
        hasher.update(b"\n");
    }

    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Confidence, Severity, SourceLocation};

    fn make_test_file(path: &str, lines: u32, hash: &str) -> FileNode {
        FileNode {
            relative_path: path.to_string(),
            language: Some("rust".to_string()),
            size_bytes: lines as u64 * 25,
            line_count: lines,
            content_hash: hash.to_string(),
            is_test: false,
        }
    }

    fn make_test_finding(id: &str, sev: Severity) -> Finding {
        Finding {
            id: id.to_string(),
            category: "security".to_string(),
            severity: sev,
            confidence: Confidence::Certain,
            title: format!("Test finding {id}"),
            description: "Desc".to_string(),
            evidence: "Ev".to_string(),
            location: Some(SourceLocation {
                file_path: "src/lib.rs".to_string(),
                start_line: Some(5),
                end_line: Some(10),
            }),
            recommendation: "Rec".to_string(),
            rule_id: "sec-01".to_string(),
        }
    }

    #[test]
    fn test_compute_sync_content_hash_determinism_and_order_independence() {
        let f1 = make_test_file("src/a.rs", 100, "hash_a");
        let f2 = make_test_file("src/b.rs", 200, "hash_b");
        let d1 = make_test_finding("find-1", Severity::High);
        let d2 = make_test_finding("find-2", Severity::Low);

        let hash_forward =
            compute_sync_content_hash(&[f1.clone(), f2.clone()], &[d1.clone(), d2.clone()]);
        let hash_reverse =
            compute_sync_content_hash(&[f2.clone(), f1.clone()], &[d2.clone(), d1.clone()]);

        assert_eq!(hash_forward, hash_reverse);
        assert_eq!(hash_forward.len(), 64);
    }

    #[test]
    fn test_compute_sync_content_hash_sensitivity() {
        let f1 = make_test_file("src/a.rs", 100, "hash_a");
        let d1 = make_test_finding("find-1", Severity::High);

        let base_hash =
            compute_sync_content_hash(std::slice::from_ref(&f1), std::slice::from_ref(&d1));

        let f1_modified = make_test_file("src/a.rs", 101, "hash_a");
        let mod_hash = compute_sync_content_hash(
            std::slice::from_ref(&f1_modified),
            std::slice::from_ref(&d1),
        );
        assert_ne!(base_hash, mod_hash);

        let d1_modified = make_test_finding("find-1", Severity::Critical);
        let mod_finding_hash = compute_sync_content_hash(
            std::slice::from_ref(&f1),
            std::slice::from_ref(&d1_modified),
        );
        assert_ne!(base_hash, mod_finding_hash);
    }
}
