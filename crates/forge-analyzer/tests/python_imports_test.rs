use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{
    build_python_architecture_graph, extract_pypi_package_name, extract_python_imports_and_exports,
    is_python_stdlib, resolve_python_import, strip_python_comments, PYTHON_STDLIB_MODULES,
};
use forge_core::FileNode;

#[test]
fn test_python_stdlib_catalog_and_lookup() -> Result<(), Box<dyn Error>> {
    assert!(PYTHON_STDLIB_MODULES.len() >= 200);

    let stdlib_samples = [
        "os",
        "sys",
        "json",
        "math",
        "pathlib",
        "typing",
        "dataclasses",
        "asyncio",
        "urllib",
        "collections",
        "itertools",
        "contextlib",
        "functools",
        "hashlib",
        "re",
        "unittest",
    ];

    for mod_name in stdlib_samples {
        assert!(
            is_python_stdlib(mod_name),
            "Expected {mod_name} to be stdlib"
        );
        assert!(
            is_python_stdlib(&format!("{mod_name}.submodule")),
            "Expected {mod_name}.submodule to be stdlib"
        );
    }

    let non_stdlib = [
        "fastapi", "pydantic", "requests", "django", "flask", "numpy", "pandas",
    ];
    for pkg in non_stdlib {
        assert!(!is_python_stdlib(pkg), "Expected {pkg} to not be stdlib");
        assert!(
            !is_python_stdlib(&format!("{pkg}.submodule")),
            "Expected {pkg}.submodule to not be stdlib"
        );
    }

    Ok(())
}

#[test]
fn test_strip_python_comments_and_docstrings() -> Result<(), Box<dyn Error>> {
    let code = r#"
# Header comment
"""
Module level docstring
with multiple lines
"""
import os # inline comment

'''
Single quote docstring
'''
def hello(): # function comment
    pass
"#;

    let stripped = strip_python_comments(code);
    assert!(!stripped.contains("Header comment"));
    assert!(!stripped.contains("Module level docstring"));
    assert!(!stripped.contains("inline comment"));
    assert!(!stripped.contains("Single quote docstring"));
    assert!(!stripped.contains("function comment"));
    assert!(stripped.contains("import os"));
    assert!(stripped.contains("def hello():"));

    Ok(())
}

#[test]
fn test_extract_python_imports_single_and_multiple() -> Result<(), Box<dyn Error>> {
    let code = r#"
import os
import sys, json
import fastapi as app
import requests.auth as req_auth
"#;

    let (imports, _) = extract_python_imports_and_exports(code);
    assert!(imports.contains(&"os".to_string()));
    assert!(imports.contains(&"sys".to_string()));
    assert!(imports.contains(&"json".to_string()));
    assert!(imports.contains(&"fastapi".to_string()));
    assert!(imports.contains(&"requests.auth".to_string()));

    Ok(())
}

#[test]
fn test_extract_python_imports_from_and_relative() -> Result<(), Box<dyn Error>> {
    let code = r#"
from typing import Optional, List, Dict
from app.models import User
from . import utils
from ..services import user_service
from ...core.db import session
"#;

    let (imports, _) = extract_python_imports_and_exports(code);
    assert!(imports.contains(&"typing".to_string()));
    assert!(imports.contains(&"app.models".to_string()));
    assert!(imports.contains(&".utils".to_string()));
    assert!(imports.contains(&"..services".to_string()));
    assert!(imports.contains(&"..services.user_service".to_string()));
    assert!(imports.contains(&"...core.db".to_string()));

    Ok(())
}

#[test]
fn test_extract_python_imports_multiline_parenthesized() -> Result<(), Box<dyn Error>> {
    let code = r#"
from app.schemas import (
    Item,
    ItemCreate,
    ItemUpdate as UpdateSchema,
)
"#;

    let (imports, _) = extract_python_imports_and_exports(code);
    assert!(imports.contains(&"app.schemas".to_string()));
    assert!(imports.contains(&"app.schemas.Item".to_string()));
    assert!(imports.contains(&"app.schemas.ItemCreate".to_string()));

    Ok(())
}

#[test]
fn test_extract_python_exports_functions_classes_and_all() -> Result<(), Box<dyn Error>> {
    let code = r#"
def sync_handler(req):
    return req

async def async_handler(req):
    return req

class ItemService:
    pass

class _PrivateHelper:
    pass

__all__ = ["PublicExport", "another_func"]
"#;

    let (_, exports) = extract_python_imports_and_exports(code);
    assert!(exports.contains(&"sync_handler".to_string()));
    assert!(exports.contains(&"async_handler".to_string()));
    assert!(exports.contains(&"ItemService".to_string()));
    assert!(!exports.contains(&"_PrivateHelper".to_string()));
    assert!(exports.contains(&"PublicExport".to_string()));
    assert!(exports.contains(&"another_func".to_string()));

    Ok(())
}

#[test]
fn test_resolve_python_import_relative_and_absolute() -> Result<(), Box<dyn Error>> {
    let known_files: BTreeSet<String> = [
        "app/__init__.py".to_string(),
        "app/routers/__init__.py".to_string(),
        "app/routers/items.py".to_string(),
        "app/services/__init__.py".to_string(),
        "app/services/user_service.py".to_string(),
        "app/services/utils.py".to_string(),
        "app/models/__init__.py".to_string(),
        "app/models/user.py".to_string(),
        "app/core/db.py".to_string(),
    ]
    .into_iter()
    .collect();

    // 1. Stdlib returns None
    assert_eq!(
        resolve_python_import("app/routers/items.py", "os", &known_files),
        None
    );
    assert_eq!(
        resolve_python_import("app/routers/items.py", "typing", &known_files),
        None
    );

    // 2. Relative from same dir: `from . import utils` inside `app/services/user_service.py`
    assert_eq!(
        resolve_python_import("app/services/user_service.py", ".utils", &known_files),
        Some("app/services/utils.py".to_string())
    );

    // 3. Relative to parent dir: `from ..services import user_service` inside `app/routers/items.py`
    assert_eq!(
        resolve_python_import(
            "app/routers/items.py",
            "..services.user_service",
            &known_files
        ),
        Some("app/services/user_service.py".to_string())
    );

    // 4. Relative to grandparent: `from ...core.db import session` inside `app/routers/sub/deep.py`
    assert_eq!(
        resolve_python_import("app/routers/sub/deep.py", "...core.db", &known_files),
        Some("app/core/db.py".to_string())
    );

    // 5. Absolute package import: `from app.models.user import User`
    assert_eq!(
        resolve_python_import("app/routers/items.py", "app.models.user", &known_files),
        Some("app/models/user.py".to_string())
    );

    // 6. Absolute package import with symbol: `from app.models.user.User`
    assert_eq!(
        resolve_python_import("app/routers/items.py", "app.models.user.User", &known_files),
        Some("app/models/user.py".to_string())
    );

    // 7. Internal import without `app.` prefix (resolved via app candidate prefix)
    assert_eq!(
        resolve_python_import(
            "app/routers/items.py",
            "services.user_service",
            &known_files
        ),
        Some("app/services/user_service.py".to_string())
    );

    // 8. Third-party external package returns None
    assert_eq!(
        resolve_python_import("app/routers/items.py", "fastapi", &known_files),
        None
    );

    Ok(())
}

#[test]
fn test_extract_pypi_package_name() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        extract_pypi_package_name("fastapi"),
        Some("fastapi".to_string())
    );
    assert_eq!(
        extract_pypi_package_name("pydantic.v1"),
        Some("pydantic".to_string())
    );
    assert_eq!(
        extract_pypi_package_name("uvicorn.workers"),
        Some("uvicorn".to_string())
    );

    // Stdlib returns None
    assert_eq!(extract_pypi_package_name("os"), None);
    assert_eq!(extract_pypi_package_name("sys"), None);
    assert_eq!(extract_pypi_package_name("typing.Optional"), None);

    // Relative returns None
    assert_eq!(extract_pypi_package_name(".utils"), None);
    assert_eq!(extract_pypi_package_name("..services"), None);

    Ok(())
}

#[test]
fn test_build_python_architecture_graph_and_cycle_detection() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_py_graph_{nanos}"));

    fs::create_dir_all(temp_dir.join("app/routers"))?;
    fs::create_dir_all(temp_dir.join("app/services"))?;
    fs::create_dir_all(temp_dir.join("app/models"))?;

    // app/routers/items.py -> imports app.services.user_service and fastapi
    fs::write(
        temp_dir.join("app/routers/items.py"),
        r#"
import fastapi
from ..services.user_service import get_user

def list_items():
    return get_user()
"#,
    )?;

    // app/services/user_service.py -> imports ..routers.items (CIRCULAR IMPORT with app/routers!)
    fs::write(
        temp_dir.join("app/services/user_service.py"),
        r#"
from ..routers.items import list_items
from ..models.user import User

def get_user():
    return User()
"#,
    )?;

    // app/models/user.py
    fs::write(
        temp_dir.join("app/models/user.py"),
        r#"
class User:
    pass
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "app/routers/items.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 120,
            line_count: 7,
            content_hash: "hash1".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "app/services/user_service.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 130,
            line_count: 8,
            content_hash: "hash2".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "app/models/user.py".to_string(),
            language: Some("python".to_string()),
            size_bytes: 40,
            line_count: 4,
            content_hash: "hash3".to_string(),
            is_test: false,
        },
    ];

    let graph = build_python_architecture_graph(&temp_dir, &files);
    let _ = fs::remove_dir_all(&temp_dir);

    let g = graph?;

    let mod_ids: Vec<&str> = g.modules.iter().map(|m| m.id.as_str()).collect();
    assert!(mod_ids.contains(&"app/routers"));
    assert!(mod_ids.contains(&"app/services"));
    assert!(mod_ids.contains(&"app/models"));

    // Verify circular dependency detected between app/routers and app/services
    assert_eq!(
        g.circular_dependencies,
        vec![vec!["app/routers".to_string(), "app/services".to_string()]]
    );

    // Verify external PyPI package extraction
    assert!(g.external_imports.contains("fastapi"));

    Ok(())
}
