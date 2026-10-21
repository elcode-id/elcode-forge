use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_cli::cli::OutputFormat;
use forge_cli::commands::dependencies::run_dependencies;
use forge_cli::commands::doctor::run_doctor;
use forge_cli::commands::graph::run_graph;
use forge_cli::commands::scan::{execute_scan, ScanOptions};

fn setup_polyglot_repo() -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_polyglot_golden_{nanos}"));

    // 1. Frontend: TypeScript / React
    let fe_src = temp_dir.join("frontend").join("src").join("components");
    fs::create_dir_all(&fe_src)?;
    fs::write(
        temp_dir.join("frontend").join("package.json"),
        r#"{
  "name": "polyglot-fe",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#,
    )?;
    fs::write(
        fe_src.join("Header.tsx"),
        r#"
import React from 'react';
export function Header() { return <h1>Polyglot App</h1>; }
"#,
    )?;

    // 2. Backend: Python / FastAPI
    let be_app = temp_dir.join("backend").join("app").join("routers");
    fs::create_dir_all(&be_app)?;
    fs::write(
        temp_dir.join("backend").join("requirements.txt"),
        "fastapi==0.100.0\nrequests>=2.28.0\n",
    )?;
    fs::write(
        be_app.join("users.py"),
        r#"
from fastapi import APIRouter
router = APIRouter()

@router.get("/users")
def get_users():
    return [{"id": 1, "name": "Alice"}]
"#,
    )?;

    // 3. Worker: Go microservice
    let go_cmd = temp_dir.join("worker").join("cmd");
    let go_pkg = temp_dir.join("worker").join("internal").join("tasks");
    fs::create_dir_all(&go_cmd)?;
    fs::create_dir_all(&go_pkg)?;
    fs::write(
        temp_dir.join("worker").join("go.mod"),
        r#"module example.com/worker

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
)
"#,
    )?;
    fs::write(
        go_cmd.join("main.go"),
        r#"
package main

import (
    "example.com/worker/internal/tasks"
    "github.com/gin-gonic/gin"
)

func main() {
    r := gin.Default()
    tasks.Register(r)
}
"#,
    )?;
    fs::write(
        go_pkg.join("tasks.go"),
        r#"
package tasks

import "github.com/gin-gonic/gin"

func Register(r *gin.Engine) {
    r.GET("/ping", func(c *gin.Context) { c.JSON(200, gin.H{"msg": "pong"}) })
}
"#,
    )?;

    // 4. Core: Rust library crate
    let rust_src = temp_dir.join("crates").join("math-core").join("src");
    fs::create_dir_all(&rust_src)?;
    fs::write(
        temp_dir.join("Cargo.toml"),
        r#"[package]
name = "math-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
log = "*"
"#,
    )?;
    fs::write(
        rust_src.join("lib.rs"),
        r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct Calculation {
    pub value: f64,
}

pub fn compute_sum(a: f64, b: f64) -> f64 {
    a + b
}
"#,
    )?;

    Ok(temp_dir)
}

#[test]
fn test_polyglot_doctor_diagnoses_all_manifests() -> Result<(), Box<dyn Error>> {
    let repo_root = setup_polyglot_repo()?;

    let report = run_doctor(&repo_root)?;
    assert!(report.repository_detected);

    let labels: Vec<&str> = report.checks.iter().map(|c| c.label.as_str()).collect();
    assert!(
        labels.contains(&"Rust manifest & lockfile hygiene"),
        "Expected Rust manifest hygiene in doctor report"
    );

    let _ = fs::remove_dir_all(&repo_root);
    Ok(())
}

#[test]
fn test_polyglot_scan_full_pipeline_and_determinism() -> Result<(), Box<dyn Error>> {
    let repo_root = setup_polyglot_repo()?;

    let opts = ScanOptions {
        repo_root: repo_root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo1, exit_code1) = execute_scan(&opts)?;

    // 1. Language detection
    assert!(repo1.detected_languages.contains(&"typescript".to_string()));
    assert!(repo1.detected_languages.contains(&"python".to_string()));
    assert!(repo1.detected_languages.contains(&"go".to_string()));
    assert!(repo1.detected_languages.contains(&"rust".to_string()));

    // 2. Dependencies across ecosystems
    let ecosystems: std::collections::BTreeSet<String> = repo1
        .dependencies
        .iter()
        .map(|d| d.ecosystem.clone())
        .collect();
    assert!(ecosystems.contains("npm"), "Expected npm dependencies");
    assert!(ecosystems.contains("pypi"), "Expected pypi dependencies");
    assert!(ecosystems.contains("go"), "Expected go dependencies");
    assert!(ecosystems.contains("cargo"), "Expected cargo dependencies");

    // 3. Module namespace separation
    let mod_ids: std::collections::BTreeSet<String> =
        repo1.modules.iter().map(|m| m.id.clone()).collect();
    assert!(mod_ids.contains("frontend/src/components"));
    assert!(mod_ids.contains("backend/app/routers"));
    assert!(mod_ids.contains("worker/cmd"));
    assert!(mod_ids.contains("worker/internal/tasks"));
    assert!(mod_ids.contains("crates/math-core"));

    // 4. Wildcard finding for log = "*"
    assert!(
        repo1
            .findings
            .iter()
            .any(|f| f.rule_id == "deps/unpinned-version" && f.evidence.contains("log")),
        "Expected wildcard finding for log dependency in Cargo.toml"
    );

    // 5. Determinism test
    let (repo2, exit_code2) = execute_scan(&opts)?;
    assert_eq!(exit_code1, exit_code2);
    assert_eq!(repo1.name, repo2.name);
    assert_eq!(repo1.detected_languages, repo2.detected_languages);
    assert_eq!(repo1.modules, repo2.modules);
    assert_eq!(repo1.dependencies, repo2.dependencies);
    assert_eq!(repo1.findings, repo2.findings);
    assert_eq!(repo1.health, repo2.health);

    let _ = fs::remove_dir_all(&repo_root);
    Ok(())
}

#[test]
fn test_polyglot_graph_mermaid_output() -> Result<(), Box<dyn Error>> {
    let repo_root = setup_polyglot_repo()?;

    let md = run_graph(&repo_root, OutputFormat::Markdown)?;
    assert!(md.contains("```mermaid\nflowchart TD"));
    assert!(md.contains("frontend/src/components"));
    assert!(md.contains("backend/app/routers"));
    assert!(md.contains("crates/math-core"));

    let _ = fs::remove_dir_all(&repo_root);
    Ok(())
}

#[test]
fn test_polyglot_dependencies_command() -> Result<(), Box<dyn Error>> {
    let repo_root = setup_polyglot_repo()?;

    let md = run_dependencies(&repo_root, OutputFormat::Markdown)?;
    assert!(md.contains("# ELCODE Forge Dependency Health"));
    assert!(md.contains("| Package | Ecosystem | Version Req | Resolved | Type |"));
    assert!(md.contains("`react`"));
    assert!(md.contains("`fastapi`"));
    assert!(md.contains("`serde`"));

    let _ = fs::remove_dir_all(&repo_root);
    Ok(())
}
