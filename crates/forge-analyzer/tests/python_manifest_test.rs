use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_analyzer::python_manifest::{
    is_python_unpinned, normalize_python_package_name, parse_pyproject_toml_content,
    parse_python_manifests, parse_requirement_line, parse_requirements_txt_content,
    read_python_lockfile_versions,
};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("eforge_py_test_{label}_{nanos}"));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_normalize_python_package_name() {
    assert_eq!(normalize_python_package_name("FastAPI"), "fastapi");
    assert_eq!(normalize_python_package_name("pydantic[email]"), "pydantic");
    assert_eq!(
        normalize_python_package_name("importlib_metadata"),
        "importlib-metadata"
    );
    assert_eq!(
        normalize_python_package_name("  uvicorn[standard]  "),
        "uvicorn"
    );
}

#[test]
fn test_parse_requirement_line() {
    assert_eq!(
        parse_requirement_line("fastapi>=0.100.0"),
        Some(("fastapi".to_string(), ">=0.100.0".to_string()))
    );

    assert_eq!(
        parse_requirement_line("pydantic[email] == 2.5.0 ; python_version >= '3.10'"),
        Some(("pydantic".to_string(), "== 2.5.0".to_string()))
    );

    assert_eq!(
        parse_requirement_line("requests"),
        Some(("requests".to_string(), "".to_string()))
    );

    assert_eq!(
        parse_requirement_line("uvicorn[standard]>=0.20.0,<0.30.0 # server"),
        Some(("uvicorn".to_string(), ">=0.20.0,<0.30.0".to_string()))
    );

    // Directives and comments should yield None
    assert_eq!(parse_requirement_line("# Comment line"), None);
    assert_eq!(parse_requirement_line("-r base.txt"), None);
    assert_eq!(parse_requirement_line("-i https://pypi.org/simple"), None);
    assert_eq!(parse_requirement_line(""), None);
}

#[test]
fn test_is_python_unpinned() {
    assert!(is_python_unpinned(""));
    assert!(is_python_unpinned("*"));
    assert!(is_python_unpinned(">=0.0.0"));
    assert!(is_python_unpinned("<2.0"));

    assert!(!is_python_unpinned(">=0.100.0"));
    assert!(!is_python_unpinned("==2.5.0"));
    assert!(!is_python_unpinned("~=1.4.0"));
    assert!(!is_python_unpinned("^2.0.0"));
}

#[test]
fn test_parse_pep621_pyproject() -> Result<(), Box<dyn Error>> {
    let toml_content = r#"
[project]
name = "demo-api"
version = "0.1.0"
dependencies = [
    "fastapi>=0.100.0",
    "pydantic[email]>=2.0",
    "unpinned-pkg",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "httpx",
]
"#;
    let mut lock = BTreeMap::new();
    lock.insert("fastapi".to_string(), "0.110.0".to_string());
    lock.insert("pytest".to_string(), "7.4.3".to_string());

    let parsed = parse_pyproject_toml_content(toml_content, &lock)?;

    assert_eq!(parsed.dependencies.len(), 5);

    let fastapi = parsed
        .dependencies
        .iter()
        .find(|d| d.name == "fastapi")
        .ok_or("missing fastapi dependency")?;
    assert_eq!(fastapi.ecosystem, "pypi");
    assert!(!fastapi.is_dev);
    assert_eq!(fastapi.version_req, ">=0.100.0");
    assert_eq!(fastapi.resolved_version.as_deref(), Some("0.110.0"));

    let pytest = parsed
        .dependencies
        .iter()
        .find(|d| d.name == "pytest")
        .ok_or("missing pytest dependency")?;
    assert!(pytest.is_dev);
    assert_eq!(pytest.resolved_version.as_deref(), Some("7.4.3"));

    assert!(parsed
        .unpinned_packages
        .contains(&"unpinned-pkg".to_string()));
    assert!(parsed.unpinned_packages.contains(&"httpx".to_string()));

    Ok(())
}

#[test]
fn test_parse_poetry_pyproject() -> Result<(), Box<dyn Error>> {
    let toml_content = r#"
[tool.poetry]
name = "poetry-service"
version = "1.0.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.100.0"
pydantic = { version = ">=2.0.0", extras = ["email"] }

[tool.poetry.group.dev.dependencies]
pytest = "^7.0"
"#;
    let lock = BTreeMap::new();
    let parsed = parse_pyproject_toml_content(toml_content, &lock)?;

    // "python" itself should be skipped
    assert_eq!(parsed.dependencies.len(), 3);
    assert!(parsed
        .dependencies
        .iter()
        .any(|d| d.name == "fastapi" && !d.is_dev));
    assert!(parsed
        .dependencies
        .iter()
        .any(|d| d.name == "pydantic" && !d.is_dev));
    assert!(parsed
        .dependencies
        .iter()
        .any(|d| d.name == "pytest" && d.is_dev));

    Ok(())
}

#[test]
fn test_parse_requirements_txt_with_duplicates() -> Result<(), Box<dyn Error>> {
    let reqs = r#"
fastapi>=0.100.0
pydantic==2.5.0
requests
# Duplicate entry:
fastapi>=0.105.0
"#;
    let lock = BTreeMap::new();
    let parsed = parse_requirements_txt_content(reqs, &lock)?;

    assert!(parsed.duplicate_packages.contains(&"fastapi".to_string()));
    assert!(parsed.unpinned_packages.contains(&"requests".to_string()));

    Ok(())
}

#[test]
fn test_read_lockfiles_and_filesystem_integration() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("integration");

    let poetry_lock = r#"
[[package]]
name = "fastapi"
version = "0.109.2"
description = "FastAPI framework"

[[package]]
name = "pydantic"
version = "2.6.1"
description = "Data validation using Python type hints"
"#;
    fs::write(root.join("poetry.lock"), poetry_lock)?;

    let lock_versions = read_python_lockfile_versions(&root);
    assert_eq!(
        lock_versions.get("fastapi").map(|s| s.as_str()),
        Some("0.109.2")
    );
    assert_eq!(
        lock_versions.get("pydantic").map(|s| s.as_str()),
        Some("2.6.1")
    );

    let requirements_txt = "fastapi>=0.100.0\npydantic>=2.0.0\n";
    fs::write(root.join("requirements.txt"), requirements_txt)?;

    let parsed = parse_python_manifests(&root)?;
    assert_eq!(parsed.dependencies.len(), 2);
    assert_eq!(parsed.dependencies[0].name, "fastapi");
    assert_eq!(
        parsed.dependencies[0].resolved_version.as_deref(),
        Some("0.109.2")
    );
    assert_eq!(parsed.dependencies[1].name, "pydantic");
    assert_eq!(
        parsed.dependencies[1].resolved_version.as_deref(),
        Some("2.6.1")
    );

    let _ = fs::remove_dir_all(&root);
    Ok(())
}
