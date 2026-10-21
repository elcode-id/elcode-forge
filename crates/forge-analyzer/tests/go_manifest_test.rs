use std::error::Error;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::{parse_go_manifest, parse_go_mod_content};

#[test]
fn test_parse_go_mod_content_multiline_require() -> Result<(), Box<dyn Error>> {
    let go_mod = r#"
module github.com/elcode-id/my-service

go 1.22.0

require (
	github.com/gin-gonic/gin v1.9.1
	github.com/google/uuid v1.6.0
	github.com/mattn/go-isatty v0.0.20 // indirect
	golang.org/x/sys v0.19.0 // indirect
)

require github.com/stretchr/testify v1.9.0
"#;

    let parsed = parse_go_mod_content(go_mod, true)?;
    assert_eq!(
        parsed.module_path.as_deref(),
        Some("github.com/elcode-id/my-service")
    );
    assert_eq!(parsed.go_version.as_deref(), Some("1.22.0"));
    assert!(parsed.has_go_sum);

    let dep_names: Vec<&str> = parsed
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert!(dep_names.contains(&"github.com/gin-gonic/gin"));
    assert!(dep_names.contains(&"github.com/google/uuid"));
    assert!(dep_names.contains(&"github.com/mattn/go-isatty"));
    assert!(dep_names.contains(&"golang.org/x/sys"));
    assert!(dep_names.contains(&"github.com/stretchr/testify"));

    let gin_dep = parsed
        .dependencies
        .iter()
        .find(|d| d.name == "github.com/gin-gonic/gin")
        .ok_or("Missing gin dependency")?;
    assert_eq!(gin_dep.version_req, "v1.9.1");
    assert_eq!(gin_dep.ecosystem, "go");
    assert!(gin_dep.is_direct);
    assert!(!gin_dep.is_dev);

    let isatty_dep = parsed
        .dependencies
        .iter()
        .find(|d| d.name == "github.com/mattn/go-isatty")
        .ok_or("Missing isatty dependency")?;
    assert_eq!(isatty_dep.version_req, "v0.0.20");
    assert!(!isatty_dep.is_direct);
    assert!(isatty_dep.is_dev);

    Ok(())
}

#[test]
fn test_parse_go_manifest_filesystem_integration() -> Result<(), Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("{e}"))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("eforge_go_manifest_{nanos}"));

    fs::create_dir_all(&temp_dir)?;

    fs::write(
        temp_dir.join("go.mod"),
        r#"
module example.com/user/demo

go 1.21

require github.com/labstack/echo/v4 v4.11.4
"#,
    )?;

    // First: without go.sum
    let parsed1 = parse_go_manifest(&temp_dir)?;
    assert_eq!(
        parsed1.module_path.as_deref(),
        Some("example.com/user/demo")
    );
    assert!(!parsed1.has_go_sum);
    assert_eq!(parsed1.dependencies.len(), 1);

    // Second: with go.sum
    fs::write(temp_dir.join("go.sum"), "dummy checksum lines")?;
    let parsed2 = parse_go_manifest(&temp_dir)?;
    assert!(parsed2.has_go_sum);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
