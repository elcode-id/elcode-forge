use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{
    build_go_architecture_graph, extract_go_external_module, extract_go_imports, is_go_stdlib,
    resolve_go_import, strip_go_comments, GO_STDLIB_ROOT_PACKAGES,
};
use forge_core::FileNode;

#[test]
fn test_go_stdlib_detection() -> Result<(), Box<dyn Error>> {
    assert!(GO_STDLIB_ROOT_PACKAGES.len() >= 40);

    let stdlib_samples = [
        "fmt",
        "net/http",
        "os",
        "os/exec",
        "io",
        "io/ioutil",
        "context",
        "sync",
        "time",
        "strings",
        "encoding/json",
        "database/sql",
        "crypto/sha256",
        "testing",
    ];

    for pkg in stdlib_samples {
        assert!(
            is_go_stdlib(pkg),
            "Expected {pkg} to be recognized as Go stdlib"
        );
    }

    let third_party = [
        "github.com/gin-gonic/gin",
        "github.com/labstack/echo/v4",
        "golang.org/x/crypto",
        "google.golang.org/grpc",
        "example.com/user/pkg",
    ];

    for pkg in third_party {
        assert!(
            !is_go_stdlib(pkg),
            "Expected {pkg} to not be recognized as Go stdlib"
        );
    }

    Ok(())
}

#[test]
fn test_strip_go_comments() -> Result<(), Box<dyn Error>> {
    let source = r#"
// Top-level package comment
package main

/*
Multi-line block comment
with multiple lines
*/
import (
    "fmt" // inline comment
)

func main() {
    /* block comment inside func */
    fmt.Println("hello")
}
"#;

    let stripped = strip_go_comments(source);
    assert!(!stripped.contains("Top-level package comment"));
    assert!(!stripped.contains("Multi-line block comment"));
    assert!(!stripped.contains("inline comment"));
    assert!(!stripped.contains("block comment inside func"));
    assert!(stripped.contains("package main"));
    assert!(stripped.contains("import ("));
    assert!(stripped.contains("fmt.Println"));

    Ok(())
}

#[test]
fn test_extract_go_imports_single_and_multiline() -> Result<(), Box<dyn Error>> {
    let source = r#"
package server

import "fmt"
import gin "github.com/gin-gonic/gin"
import _ "github.com/lib/pq"

import (
    "net/http"
    "context"
    app_svc "github.com/elcode-id/my-service/internal/services"
    "github.com/google/uuid"
)
"#;

    let imports = extract_go_imports(source);
    assert!(imports.contains(&"fmt".to_string()));
    assert!(imports.contains(&"github.com/gin-gonic/gin".to_string()));
    assert!(imports.contains(&"github.com/lib/pq".to_string()));
    assert!(imports.contains(&"net/http".to_string()));
    assert!(imports.contains(&"context".to_string()));
    assert!(imports.contains(&"github.com/elcode-id/my-service/internal/services".to_string()));
    assert!(imports.contains(&"github.com/google/uuid".to_string()));

    Ok(())
}

#[test]
fn test_resolve_go_import_and_extract_external_module() -> Result<(), Box<dyn Error>> {
    let known_files: BTreeSet<String> = [
        "cmd/server/main.go".to_string(),
        "internal/handlers/user.go".to_string(),
        "internal/services/user_service.go".to_string(),
        "pkg/models/user.go".to_string(),
    ]
    .into_iter()
    .collect();

    let module_path = "github.com/elcode-id/my-service";

    // 1. Stdlib returns None
    assert_eq!(
        resolve_go_import("cmd/server/main.go", "fmt", Some(module_path), &known_files),
        None
    );
    assert_eq!(
        resolve_go_import(
            "cmd/server/main.go",
            "net/http",
            Some(module_path),
            &known_files
        ),
        None
    );

    // 2. Internal package matching module_path
    assert_eq!(
        resolve_go_import(
            "cmd/server/main.go",
            "github.com/elcode-id/my-service/internal/handlers",
            Some(module_path),
            &known_files
        ),
        Some("internal/handlers/user.go".to_string())
    );

    // 3. Third-party package returns None from resolver
    assert_eq!(
        resolve_go_import(
            "cmd/server/main.go",
            "github.com/gin-gonic/gin",
            Some(module_path),
            &known_files
        ),
        None
    );

    // 4. External module extraction
    assert_eq!(
        extract_go_external_module("github.com/gin-gonic/gin/binding", Some(module_path)),
        Some("github.com/gin-gonic/gin".to_string())
    );
    assert_eq!(
        extract_go_external_module("golang.org/x/crypto/bcrypt", Some(module_path)),
        Some("golang.org/x/crypto".to_string())
    );
    assert_eq!(extract_go_external_module("fmt", Some(module_path)), None);
    assert_eq!(
        extract_go_external_module(
            "github.com/elcode-id/my-service/internal/handlers",
            Some(module_path)
        ),
        None
    );

    Ok(())
}

#[test]
fn test_build_go_architecture_graph_and_cycle_detection() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_go_graph_{nanos}"));

    fs::create_dir_all(temp_dir.join("internal/handlers"))?;
    fs::create_dir_all(temp_dir.join("internal/services"))?;
    fs::create_dir_all(temp_dir.join("cmd/server"))?;

    // cmd/server/main.go
    fs::write(
        temp_dir.join("cmd/server/main.go"),
        r#"
package main

import (
    "github.com/gin-gonic/gin"
    "example.com/demo/internal/handlers"
)

func main() {
    _ = handlers.HandleUser
}
"#,
    )?;

    // internal/handlers/user.go -> imports internal/services
    fs::write(
        temp_dir.join("internal/handlers/user.go"),
        r#"
package handlers

import "example.com/demo/internal/services"

func HandleUser() {
    _ = services.GetUser
}
"#,
    )?;

    // internal/services/user_service.go -> imports internal/handlers (CIRCULAR IMPORT!)
    fs::write(
        temp_dir.join("internal/services/user_service.go"),
        r#"
package services

import "example.com/demo/internal/handlers"

func GetUser() {
    _ = handlers.HandleUser
}
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "cmd/server/main.go".to_string(),
            language: Some("go".to_string()),
            size_bytes: 120,
            line_count: 10,
            content_hash: "hash1".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "internal/handlers/user.go".to_string(),
            language: Some("go".to_string()),
            size_bytes: 110,
            line_count: 8,
            content_hash: "hash2".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "internal/services/user_service.go".to_string(),
            language: Some("go".to_string()),
            size_bytes: 110,
            line_count: 8,
            content_hash: "hash3".to_string(),
            is_test: false,
        },
    ];

    let graph = build_go_architecture_graph(&temp_dir, &files, Some("example.com/demo"))?;
    let _ = fs::remove_dir_all(&temp_dir);

    let mod_ids: Vec<&str> = graph.modules.iter().map(|m| m.id.as_str()).collect();
    assert!(mod_ids.contains(&"cmd/server"));
    assert!(mod_ids.contains(&"internal/handlers"));
    assert!(mod_ids.contains(&"internal/services"));

    // Verify circular dependency detected between internal/handlers and internal/services
    assert_eq!(
        graph.circular_dependencies,
        vec![vec![
            "internal/handlers".to_string(),
            "internal/services".to_string()
        ]]
    );

    // Verify external import extraction
    assert!(graph.external_imports.contains("github.com/gin-gonic/gin"));

    Ok(())
}
