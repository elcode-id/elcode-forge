use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{
    build_rust_architecture_graph, expand_use_tree, extract_rust_external_crate,
    extract_rust_mods_and_uses, infer_rust_module_boundary, is_rust_std, resolve_rust_import,
    strip_rust_comments, RUST_STD_CRATES,
};
use forge_core::FileNode;

#[test]
fn test_rust_stdlib_filtering() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        RUST_STD_CRATES,
        &["std", "core", "alloc", "proc_macro", "test"]
    );

    for s in ["std", "core", "alloc", "proc_macro", "test"] {
        assert!(is_rust_std(s), "Expected {s} to be recognized as Rust std");
        assert!(
            is_rust_std(&format!("{s}::fmt::Debug")),
            "Expected subpath in {s} to be std"
        );
    }

    for ext in ["serde", "tokio", "anyhow", "forge_core", "clap"] {
        assert!(
            !is_rust_std(ext),
            "Expected {ext} to not be recognized as Rust std"
        );
    }

    Ok(())
}

#[test]
fn test_strip_rust_comments() -> Result<(), Box<dyn Error>> {
    let source = r##"
// Top-level comment
/// Inner doc comment
//! Module doc comment
fn main() {
    /* Multi-line
       block comment
    */
    let s = "hello // not comment /* also not */";
    let raw = r#"raw /* comment */ string"#;
    /* Outer /* Nested */ block */
    println!("{}", s);
}
"##;

    let stripped = strip_rust_comments(source);
    assert!(!stripped.contains("Top-level comment"));
    assert!(!stripped.contains("Inner doc comment"));
    assert!(!stripped.contains("Module doc comment"));
    assert!(!stripped.contains("Multi-line"));
    assert!(!stripped.contains("Nested"));
    assert!(stripped.contains("fn main()"));
    assert!(stripped.contains("hello // not comment /* also not */"));
    assert!(stripped.contains(r##"raw /* comment */ string"##));

    Ok(())
}

#[test]
fn test_expand_use_tree() -> Result<(), Box<dyn Error>> {
    // 1. Simple
    assert_eq!(
        expand_use_tree("", "crate::foo::bar"),
        vec!["crate::foo::bar".to_string()]
    );

    // 2. Grouped
    assert_eq!(
        expand_use_tree("", "crate::foo::{bar, baz}"),
        vec!["crate::foo::bar".to_string(), "crate::foo::baz".to_string()]
    );

    // 3. Self
    assert_eq!(
        expand_use_tree("", "crate::foo::{self, bar}"),
        vec!["crate::foo".to_string(), "crate::foo::bar".to_string()]
    );

    // 4. Nested
    assert_eq!(
        expand_use_tree("", "std::{collections::{BTreeMap, BTreeSet}, sync::Arc}"),
        vec![
            "std::collections::BTreeMap".to_string(),
            "std::collections::BTreeSet".to_string(),
            "std::sync::Arc".to_string(),
        ]
    );

    // 5. Aliased
    assert_eq!(
        expand_use_tree("", "serde::{Serialize as Ser, Deserialize}"),
        vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string()
        ]
    );

    Ok(())
}

#[test]
fn test_extract_rust_mods_and_uses() -> Result<(), Box<dyn Error>> {
    let source = r#"
mod alpha;
pub mod beta;
pub(crate) mod gamma;

// Inline module should not be treated as file submodule
mod tests {
    use super::*;
}

use std::path::Path;
use serde::{Deserialize, Serialize};

pub use crate::{
    scanner::scan_repository,
    graph::build_rust_architecture_graph,
};
"#;

    let (mods, uses) = extract_rust_mods_and_uses(source);

    assert_eq!(
        mods,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );

    assert!(uses.contains(&"std::path::Path".to_string()));
    assert!(uses.contains(&"serde::Deserialize".to_string()));
    assert!(uses.contains(&"serde::Serialize".to_string()));
    assert!(uses.contains(&"crate::scanner::scan_repository".to_string()));
    assert!(uses.contains(&"crate::graph::build_rust_architecture_graph".to_string()));

    Ok(())
}

#[test]
fn test_resolve_rust_import_internal_and_external() -> Result<(), Box<dyn Error>> {
    let mut known_files = BTreeSet::new();
    known_files.insert("crates/forge-analyzer/src/lib.rs".to_string());
    known_files.insert("crates/forge-analyzer/src/graph.rs".to_string());
    known_files.insert("crates/forge-analyzer/src/scanner.rs".to_string());
    known_files.insert("crates/forge-core/src/lib.rs".to_string());

    let importer = "crates/forge-analyzer/src/lib.rs";

    // 1. Resolve declared mod
    let resolved_mod = resolve_rust_import(importer, "mod::graph", &known_files);
    assert_eq!(
        resolved_mod.as_deref(),
        Some("crates/forge-analyzer/src/graph.rs")
    );

    // 2. Resolve crate::
    let resolved_crate =
        resolve_rust_import(importer, "crate::scanner::scan_repository", &known_files);
    assert_eq!(
        resolved_crate.as_deref(),
        Some("crates/forge-analyzer/src/scanner.rs")
    );

    // 3. Resolve workspace crate
    let resolved_ws = resolve_rust_import(importer, "forge_core::ForgeError", &known_files);
    assert_eq!(resolved_ws.as_deref(), Some("crates/forge-core/src/lib.rs"));

    // 4. Stdlib returns None
    assert_eq!(
        resolve_rust_import(importer, "std::path::Path", &known_files),
        None
    );

    // 5. External crate returns None
    assert_eq!(
        resolve_rust_import(importer, "serde::Deserialize", &known_files),
        None
    );

    // 6. External crate extractor
    assert_eq!(
        extract_rust_external_crate("serde::Deserialize"),
        Some("serde".to_string())
    );
    assert_eq!(
        extract_rust_external_crate("tokio::sync::Mutex"),
        Some("tokio".to_string())
    );
    assert_eq!(extract_rust_external_crate("std::sync::Arc"), None);
    assert_eq!(extract_rust_external_crate("crate::scanner"), None);

    Ok(())
}

#[test]
fn test_infer_rust_module_boundary() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        infer_rust_module_boundary("crates/forge-cli/src/commands/doctor.rs"),
        ("crates/forge-cli".to_string(), "forge-cli".to_string())
    );
    assert_eq!(
        infer_rust_module_boundary("crates/forge-analyzer/src/lib.rs"),
        (
            "crates/forge-analyzer".to_string(),
            "forge-analyzer".to_string()
        )
    );
    assert_eq!(
        infer_rust_module_boundary("src/handlers/auth.rs"),
        ("src/handlers".to_string(), "handlers".to_string())
    );
    assert_eq!(
        infer_rust_module_boundary("src/main.rs"),
        ("src".to_string(), "src".to_string())
    );

    Ok(())
}

#[test]
fn test_build_rust_architecture_graph_and_cycle_detection() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_rust_graph_{nanos}"));

    let mod_a_dir = temp_dir.join("src").join("mod_a");
    let mod_b_dir = temp_dir.join("src").join("mod_b");
    fs::create_dir_all(&mod_a_dir)?;
    fs::create_dir_all(&mod_b_dir)?;

    fs::write(
        mod_a_dir.join("mod.rs"),
        r#"
use crate::mod_b::process_b;
use serde::Serialize;

pub fn process_a() {
    process_b();
}
"#,
    )?;

    fs::write(
        mod_b_dir.join("mod.rs"),
        r#"
use crate::mod_a::process_a;

pub fn process_b() {
    process_a();
}
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "src/mod_a/mod.rs".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 100,
            line_count: 10,
            content_hash: "hash_a".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "src/mod_b/mod.rs".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 100,
            line_count: 10,
            content_hash: "hash_b".to_string(),
            is_test: false,
        },
    ];

    let graph = build_rust_architecture_graph(&temp_dir, &files)?;

    let mod_ids: Vec<&str> = graph.modules.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(mod_ids, vec!["src/mod_a", "src/mod_b"]);

    // Circular dependency detected between src/mod_a and src/mod_b
    assert_eq!(
        graph.circular_dependencies,
        vec![vec!["src/mod_a".to_string(), "src/mod_b".to_string()]]
    );

    // External crate serde extracted
    assert!(graph.external_imports.contains("serde"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
