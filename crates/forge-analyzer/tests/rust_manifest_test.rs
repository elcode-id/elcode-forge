use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{parse_cargo_toml_content, parse_rust_workspace, read_cargo_lock_versions};

#[test]
fn test_parse_cargo_toml_content_dependencies() -> Result<(), Box<dyn Error>> {
    let toml_content = r#"
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = "1.38"
wildcard-dep = "*"
local-helper = { path = "../helper" }

[dev-dependencies]
tempfile = "3.10"

[build-dependencies]
cc = "1.0"
"#;

    let mut lock_versions = BTreeMap::new();
    lock_versions.insert("serde".to_string(), "1.0.210".to_string());
    lock_versions.insert("tokio".to_string(), "1.38.1".to_string());
    lock_versions.insert("tempfile".to_string(), "3.10.1".to_string());

    let (deps, wildcards, paths) = parse_cargo_toml_content(toml_content, &lock_versions)?;

    assert_eq!(wildcards, vec!["wildcard-dep".to_string()]);
    assert_eq!(paths, vec!["local-helper".to_string()]);

    let serde_dep = deps
        .iter()
        .find(|d| d.name == "serde")
        .ok_or("Missing serde dependency")?;
    assert_eq!(serde_dep.version_req, "1.0");
    assert_eq!(serde_dep.resolved_version.as_deref(), Some("1.0.210"));
    assert_eq!(serde_dep.ecosystem, "cargo");
    assert!(!serde_dep.is_dev);

    let tempfile_dep = deps
        .iter()
        .find(|d| d.name == "tempfile")
        .ok_or("Missing tempfile dev-dependency")?;
    assert!(tempfile_dep.is_dev);
    assert_eq!(tempfile_dep.resolved_version.as_deref(), Some("3.10.1"));

    let cc_dep = deps
        .iter()
        .find(|d| d.name == "cc")
        .ok_or("Missing cc build-dependency")?;
    assert!(cc_dep.is_dev);

    Ok(())
}

#[test]
fn test_parse_rust_workspace_filesystem_integration() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_rust_workspace_{nanos}"));

    fs::create_dir_all(temp_dir.join("crates").join("pkg-a"))?;
    fs::create_dir_all(temp_dir.join("crates").join("pkg-b"))?;

    // Root Cargo.toml
    fs::write(
        temp_dir.join("Cargo.toml"),
        r#"
[workspace]
members = [
    "crates/*",
]

[workspace.dependencies]
serde = "1.0"
"#,
    )?;

    // Member pkg-a
    fs::write(
        temp_dir.join("crates").join("pkg-a").join("Cargo.toml"),
        r#"
[package]
name = "pkg-a"
version = "0.1.0"

[dependencies]
serde = { workspace = true }
pkg-b = { path = "../pkg-b" }
"#,
    )?;

    // Member pkg-b
    fs::write(
        temp_dir.join("crates").join("pkg-b").join("Cargo.toml"),
        r#"
[package]
name = "pkg-b"
version = "0.1.0"

[dependencies]
anyhow = "*"
"#,
    )?;

    // 1. Without Cargo.lock
    let parsed1 = parse_rust_workspace(&temp_dir)?;
    assert!(parsed1.is_workspace);
    assert!(!parsed1.has_cargo_lock);
    assert_eq!(
        parsed1.workspace_members,
        vec!["crates/pkg-a".to_string(), "crates/pkg-b".to_string()]
    );
    assert!(parsed1
        .wildcard_dependencies
        .contains(&"anyhow".to_string()));
    assert!(parsed1.path_dependencies.contains(&"pkg-b".to_string()));

    // 2. With Cargo.lock
    fs::write(
        temp_dir.join("Cargo.lock"),
        r#"
version = 3

[[package]]
name = "serde"
version = "1.0.210"

[[package]]
name = "anyhow"
version = "1.0.86"
"#,
    )?;

    let parsed2 = parse_rust_workspace(&temp_dir)?;
    assert!(parsed2.has_cargo_lock);
    let serde_dep = parsed2
        .dependencies
        .iter()
        .find(|d| d.name == "serde")
        .ok_or("Missing serde in parsed workspace")?;
    assert_eq!(serde_dep.resolved_version.as_deref(), Some("1.0.210"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_self_dogfooding_elcode_forge_workspace() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    let canonical_root = fs::canonicalize(&repo_root)?;
    let parsed = parse_rust_workspace(&canonical_root)?;

    assert!(parsed.is_workspace);
    assert!(parsed.has_cargo_lock);

    // Verify workspace members exist
    assert!(
        parsed
            .workspace_members
            .iter()
            .any(|m| m.contains("forge-core")),
        "Expected workspace members to contain forge-core"
    );
    assert!(
        parsed
            .workspace_members
            .iter()
            .any(|m| m.contains("forge-analyzer")),
        "Expected workspace members to contain forge-analyzer"
    );
    assert!(
        parsed
            .workspace_members
            .iter()
            .any(|m| m.contains("forge-cli")),
        "Expected workspace members to contain forge-cli"
    );

    // Verify Cargo.lock versions resolved
    let lock_versions = read_cargo_lock_versions(&canonical_root);
    assert!(
        !lock_versions.is_empty(),
        "Cargo.lock should contain resolved packages"
    );
    assert!(lock_versions.contains_key("serde"));
    assert!(lock_versions.contains_key("toml"));

    // Verify dependencies list contains core dependencies
    let dep_names: Vec<&str> = parsed
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert!(dep_names.contains(&"serde"));
    assert!(dep_names.contains(&"thiserror"));

    Ok(())
}
