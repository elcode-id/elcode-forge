use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{analyze_polyglot_dependencies, build_polyglot_architecture_graph};
use forge_core::FileNode;

#[test]
fn test_polyglot_architecture_graph_combines_all_languages() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_polyglot_graph_{nanos}"));

    // 1. Frontend TS files
    let ts_comp = temp_dir.join("frontend").join("src").join("components");
    let ts_serv = temp_dir.join("frontend").join("src").join("services");
    fs::create_dir_all(&ts_comp)?;
    fs::create_dir_all(&ts_serv)?;
    fs::write(
        ts_comp.join("Card.tsx"),
        r#"
import React from 'react';
import { fetchCardData } from '../services/api';
export function Card() { return <div>Card</div>; }
"#,
    )?;
    fs::write(
        ts_serv.join("api.ts"),
        r#"
import axios from 'axios';
export function fetchCardData() { return axios.get('/api/card'); }
"#,
    )?;

    // 2. Backend Python files
    let py_router = temp_dir.join("backend").join("app").join("routers");
    let py_service = temp_dir.join("backend").join("app").join("services");
    fs::create_dir_all(&py_router)?;
    fs::create_dir_all(&py_service)?;
    fs::write(
        py_router.join("items.py"),
        r#"
from fastapi import APIRouter
from backend.app.services.compute import calculate_score

router = APIRouter()
"#,
    )?;
    fs::write(
        py_service.join("compute.py"),
        r#"
import pydantic
def calculate_score(val: int) -> int:
    return val * 42
"#,
    )?;

    // 3. Services Go worker files
    let go_cmd = temp_dir.join("services").join("worker").join("cmd");
    let go_proc = temp_dir
        .join("services")
        .join("worker")
        .join("internal")
        .join("processor");
    fs::create_dir_all(&go_cmd)?;
    fs::create_dir_all(&go_proc)?;
    fs::write(
        temp_dir.join("services").join("worker").join("go.mod"),
        r#"
module example.com/worker

go 1.21

require (
    github.com/google/uuid v1.4.0
)
"#,
    )?;
    fs::write(
        go_cmd.join("main.go"),
        r#"
package main

import (
    "example.com/worker/internal/processor"
    "github.com/google/uuid"
)

func main() {
    id := uuid.New().String()
    processor.Process(id)
}
"#,
    )?;
    fs::write(
        go_proc.join("job.go"),
        r#"
package processor

import "fmt"

func Process(id string) {
    fmt.Println(id)
}
"#,
    )?;

    // 4. Core Rust engine files
    let rust_core = temp_dir.join("crates").join("core-engine").join("src");
    fs::create_dir_all(&rust_core)?;
    fs::write(
        rust_core.join("lib.rs"),
        r#"
pub mod math;
use serde::Serialize;

#[derive(Serialize)]
pub struct EngineConfig {
    pub threads: usize,
}
"#,
    )?;
    fs::write(
        rust_core.join("math.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "frontend/src/components/Card.tsx".to_string(),
            language: Some("typescript".to_string()),
            size_bytes: 120,
            line_count: 5,
            content_hash: "h1".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "frontend/src/services/api.ts".to_string(),
            language: Some("typescript".to_string()),
            size_bytes: 110,
            line_count: 4,
            content_hash: "h2".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "backend/app/routers/items.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 140,
            line_count: 6,
            content_hash: "h3".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "backend/app/services/compute.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 100,
            line_count: 5,
            content_hash: "h4".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "services/worker/go.mod".to_string(),
            language: None,
            size_bytes: 80,
            line_count: 8,
            content_hash: "h5".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "services/worker/cmd/main.go".to_string(),
            language: Some("go".to_string()),
            size_bytes: 160,
            line_count: 14,
            content_hash: "h6".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "services/worker/internal/processor/job.go".to_string(),
            language: Some("go".to_string()),
            size_bytes: 90,
            line_count: 8,
            content_hash: "h7".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "crates/core-engine/src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 130,
            line_count: 9,
            content_hash: "h8".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "crates/core-engine/src/math.rs".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 60,
            line_count: 4,
            content_hash: "h9".to_string(),
            is_test: false,
        },
    ];

    let graph = build_polyglot_architecture_graph(&temp_dir, &files)?;

    let mod_ids: BTreeSet<String> = graph.modules.iter().map(|m| m.id.clone()).collect();
    assert!(
        mod_ids.contains("frontend/src/components"),
        "Expected frontend components"
    );
    assert!(
        mod_ids.contains("frontend/src/services"),
        "Expected frontend services"
    );
    assert!(
        mod_ids.contains("backend/app/routers"),
        "Expected backend routers"
    );
    assert!(
        mod_ids.contains("backend/app/services"),
        "Expected backend services"
    );
    assert!(
        mod_ids.contains("services/worker/cmd"),
        "Expected Go worker cmd"
    );
    assert!(
        mod_ids.contains("services/worker/internal/processor"),
        "Expected Go processor"
    );
    assert!(
        mod_ids.contains("crates/core-engine"),
        "Expected Rust core-engine"
    );

    // Check external imports across languages
    assert!(
        graph.external_imports.contains("react"),
        "Expected npm react"
    );
    assert!(
        graph.external_imports.contains("axios"),
        "Expected npm axios"
    );
    assert!(
        graph.external_imports.contains("fastapi"),
        "Expected PyPI fastapi"
    );
    assert!(
        graph.external_imports.contains("pydantic"),
        "Expected PyPI pydantic"
    );
    assert!(
        graph.external_imports.contains("github.com/google/uuid"),
        "Expected Go uuid"
    );
    assert!(
        graph.external_imports.contains("serde"),
        "Expected Cargo serde"
    );

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_polyglot_dependencies_aggregation_and_health() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_polyglot_deps_{nanos}"));

    // 1. Frontend package.json
    let fe_dir = temp_dir.join("frontend");
    fs::create_dir_all(&fe_dir)?;
    fs::write(
        fe_dir.join("package.json"),
        r#"{
  "name": "polyglot-fe",
  "dependencies": {
    "react": "^18.2.0",
    "lodash": "*"
  }
}"#,
    )?;

    // 2. Backend requirements.txt
    let be_dir = temp_dir.join("backend");
    fs::create_dir_all(&be_dir)?;
    fs::write(
        be_dir.join("requirements.txt"),
        r#"fastapi==0.100.0
requests>=2.0.0
"#,
    )?;

    // 3. Go worker go.mod
    let go_dir = temp_dir.join("services").join("worker");
    fs::create_dir_all(&go_dir)?;
    fs::write(
        go_dir.join("go.mod"),
        r#"module example.com/worker

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
)
"#,
    )?;

    // 4. Rust Cargo.toml
    fs::write(
        temp_dir.join("Cargo.toml"),
        r#"[package]
name = "polyglot-root"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
log = "*"
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "frontend/package.json".to_string(),
            language: None,
            size_bytes: 100,
            line_count: 7,
            content_hash: "d1".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "backend/requirements.txt".to_string(),
            language: None,
            size_bytes: 40,
            line_count: 2,
            content_hash: "d2".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "services/worker/go.mod".to_string(),
            language: None,
            size_bytes: 70,
            line_count: 7,
            content_hash: "d3".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "Cargo.toml".to_string(),
            language: None,
            size_bytes: 120,
            line_count: 9,
            content_hash: "d4".to_string(),
            is_test: false,
        },
    ];

    let external_imports = BTreeSet::from([
        "react".to_string(),
        "fastapi".to_string(),
        "requests".to_string(),
        "github.com/gin-gonic/gin".to_string(),
        "serde".to_string(),
    ]);

    let report = analyze_polyglot_dependencies(&temp_dir, &files, &external_imports)?;

    // Check all ecosystems present
    let ecosystems: BTreeSet<String> = report
        .dependencies
        .iter()
        .map(|d| d.ecosystem.clone())
        .collect();
    assert!(ecosystems.contains("npm"), "Expected npm dependencies");
    assert!(ecosystems.contains("pypi"), "Expected pypi dependencies");
    assert!(ecosystems.contains("go"), "Expected go dependencies");
    assert!(ecosystems.contains("cargo"), "Expected cargo dependencies");

    // Check wildcard / unpinned warnings
    assert!(
        report
            .wildcard_or_unpinned
            .iter()
            .any(|p| p == "lodash" || p == "log" || p == "requests"),
        "Expected wildcard or unpinned detection for lodash, log, or requests"
    );

    // Check findings contain wildcard alerts
    let rule_ids: BTreeSet<String> = report.findings.iter().map(|f| f.rule_id.clone()).collect();
    assert!(
        rule_ids.contains("deps/unpinned-version"),
        "Expected deps/unpinned-version finding"
    );

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_polyglot_cycle_detection_in_subprojects() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_polyglot_cycles_{nanos}"));

    // Create intentional cycle in Python service: backend/app/mod_a <-> backend/app/mod_b
    let py_a = temp_dir.join("backend").join("app").join("mod_a");
    let py_b = temp_dir.join("backend").join("app").join("mod_b");
    fs::create_dir_all(&py_a)?;
    fs::create_dir_all(&py_b)?;

    fs::write(
        py_a.join("__init__.py"),
        "from backend.app.mod_b import func_b\ndef func_a(): return func_b()\n",
    )?;
    fs::write(
        py_b.join("__init__.py"),
        "from backend.app.mod_a import func_a\ndef func_b(): return func_a()\n",
    )?;

    let files = vec![
        FileNode {
            relative_path: "backend/app/mod_a/__init__.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 80,
            line_count: 2,
            content_hash: "cy1".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "backend/app/mod_b/__init__.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 80,
            line_count: 2,
            content_hash: "cy2".to_string(),
            is_test: false,
        },
    ];

    let graph = build_polyglot_architecture_graph(&temp_dir, &files)?;

    assert_eq!(
        graph.circular_dependencies,
        vec![vec![
            "backend/app/mod_a".to_string(),
            "backend/app/mod_b".to_string(),
        ]]
    );

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
