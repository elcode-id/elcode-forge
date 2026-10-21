use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::php_imports::{
    build_php_architecture_graph, extract_composer_package_name, extract_php_namespace_and_uses,
    infer_php_module_boundary, is_php_builtin, resolve_php_import, strip_php_comments,
};
use forge_core::FileNode;

fn create_temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[test]
fn test_strip_php_comments() {
    let source = r#"<?php
// Single line comment
# Shell style comment
/* Block comment */
$msg = "Hello // not a comment";
$query = 'Select * # not a comment';
"#;
    let stripped = strip_php_comments(source);
    assert!(!stripped.contains("Single line comment"));
    assert!(!stripped.contains("Shell style comment"));
    assert!(!stripped.contains("Block comment"));
    assert!(stripped.contains("$msg = \"Hello // not a comment\";"));
    assert!(stripped.contains("$query = 'Select * # not a comment';"));
}

#[test]
fn test_is_php_builtin() {
    assert!(is_php_builtin("stdClass"));
    assert!(is_php_builtin("DateTime"));
    assert!(is_php_builtin("Exception"));
    assert!(is_php_builtin("PDO"));
    assert!(is_php_builtin("\\Closure"));

    assert!(!is_php_builtin("App\\Models\\User"));
    assert!(!is_php_builtin("GuzzleHttp\\Client"));
}

#[test]
fn test_extract_php_namespace_and_uses() {
    let source = r#"<?php
namespace App\Http\Controllers;

use App\Models\User as UserModel;
use App\Services\{UserService, AuthService as Auth};
use Illuminate\Support\Facades\DB;
use stdClass; // Should be filtered
use DateTime; // Should be filtered

class UserController extends Controller {
    public function show() {
        return DB::table('users')->get();
    }
}
"#;

    let (ns, uses) = extract_php_namespace_and_uses(source);
    assert_eq!(ns.as_deref(), Some("App\\Http\\Controllers"));

    assert!(uses.contains(&"App\\Models\\User".to_string()));
    assert!(uses.contains(&"App\\Services\\UserService".to_string()));
    assert!(uses.contains(&"App\\Services\\AuthService".to_string()));
    assert!(uses.contains(&"Illuminate\\Support\\Facades\\DB".to_string()));
    assert!(!uses.contains(&"stdClass".to_string()));
    assert!(!uses.contains(&"DateTime".to_string()));
}

#[test]
fn test_infer_php_module_boundary() {
    assert_eq!(
        infer_php_module_boundary("app/Http/Controllers/UserController.php"),
        (
            "app/Http/Controllers".to_string(),
            "Controllers".to_string()
        )
    );
    assert_eq!(
        infer_php_module_boundary("app/Models/User.php"),
        ("app/Models".to_string(), "Models".to_string())
    );
    assert_eq!(
        infer_php_module_boundary("app/Services/UserService.php"),
        ("app/Services".to_string(), "Services".to_string())
    );
    assert_eq!(
        infer_php_module_boundary("tests/Feature/UserTest.php"),
        ("tests/Feature".to_string(), "Feature".to_string())
    );
}

#[test]
fn test_resolve_php_import_and_extract_package() {
    let mut psr4 = BTreeMap::new();
    psr4.insert("App\\".to_string(), "app".to_string());

    let mut known = BTreeSet::new();
    known.insert("app/Services/UserService.php".to_string());
    known.insert("app/Models/User.php".to_string());

    let resolved = resolve_php_import("App\\Services\\UserService", &psr4, &known);
    assert_eq!(resolved.as_deref(), Some("app/Services/UserService.php"));

    let external = resolve_php_import("Illuminate\\Support\\Facades\\DB", &psr4, &known);
    assert_eq!(external, None);

    assert_eq!(
        extract_composer_package_name("Illuminate\\Support\\Facades\\DB"),
        Some("laravel/framework".to_string())
    );
    assert_eq!(
        extract_composer_package_name("GuzzleHttp\\Client"),
        Some("guzzlehttp/guzzle".to_string())
    );
    assert_eq!(
        extract_composer_package_name("Symfony\\Component\\HttpFoundation\\Request"),
        Some("symfony/http-foundation".to_string())
    );
}

#[test]
fn test_build_php_architecture_graph_and_cycle_detection() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_dir("php_graph_test")?;

    // Create app/Services/A.php and app/Services/B.php with mutual imports
    let srv_dir = temp_dir.join("app").join("Services");
    let ctl_dir = temp_dir.join("app").join("Http").join("Controllers");
    fs::create_dir_all(&srv_dir)?;
    fs::create_dir_all(&ctl_dir)?;

    fs::write(
        srv_dir.join("ServiceA.php"),
        r#"<?php
namespace App\Services;
use App\Http\Controllers\UserController;
class ServiceA {}
"#,
    )?;

    fs::write(
        ctl_dir.join("UserController.php"),
        r#"<?php
namespace App\Http\Controllers;
use App\Services\ServiceA;
use GuzzleHttp\Client;
class UserController {}
"#,
    )?;

    let files = vec![
        FileNode {
            relative_path: "app/Services/ServiceA.php".to_string(),
            language: Some("php".to_string()),
            size_bytes: 100,
            line_count: 5,
            content_hash: "a".to_string(),
            is_test: false,
        },
        FileNode {
            relative_path: "app/Http/Controllers/UserController.php".to_string(),
            language: Some("php".to_string()),
            size_bytes: 100,
            line_count: 6,
            content_hash: "b".to_string(),
            is_test: false,
        },
    ];

    let graph = build_php_architecture_graph(&temp_dir, &files)?;
    assert_eq!(graph.modules.len(), 2);

    // Verify circular dependency between app/Http/Controllers and app/Services
    assert_eq!(
        graph.circular_dependencies,
        vec![vec![
            "app/Http/Controllers".to_string(),
            "app/Services".to_string(),
        ]]
    );

    // Verify external import extraction
    assert!(graph.external_imports.contains("guzzlehttp/guzzle"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
