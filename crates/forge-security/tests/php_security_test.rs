use std::error::Error;

use forge_security::php_patterns::{
    scan_php_unsafe_patterns, RULE_PHP_COMMAND_INJECTION, RULE_PHP_CREATE_FUNCTION,
    RULE_PHP_INSECURE_UNSERIALIZE, RULE_PHP_UNSAFE_EVAL,
};

#[test]
fn test_php_unsafe_eval_detected() -> Result<(), Box<dyn Error>> {
    let source = r#"<?php
$code = $_GET['code'];
eval($code);
"#;
    let findings = scan_php_unsafe_patterns("app/Http/Controllers/TestController.php", source);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_PHP_UNSAFE_EVAL);

    let loc = findings[0]
        .location
        .as_ref()
        .ok_or("Expected source location")?;
    assert_eq!(loc.start_line, Some(3));
    Ok(())
}

#[test]
fn test_php_insecure_unserialize_detected() {
    let source = r#"<?php
$data = $_COOKIE['session'];
$user = unserialize($data);
"#;
    let findings = scan_php_unsafe_patterns("app/Services/SessionService.php", source);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_PHP_INSECURE_UNSERIALIZE);
}

#[test]
fn test_php_command_injection_detected() {
    let source = r#"<?php
$ip = $_POST['ip'];
exec("ping -c 4 " . $ip);
shell_exec("ls -la");
system("id");
passthru("cat /etc/passwd");
"#;
    let findings = scan_php_unsafe_patterns("app/Services/NetworkService.php", source);
    assert_eq!(findings.len(), 4);
    for f in &findings {
        assert_eq!(f.rule_id, RULE_PHP_COMMAND_INJECTION);
    }
}

#[test]
fn test_php_create_function_detected() {
    let source = r#"<?php
$fn = create_function('$a,$b', 'return $a + $b;');
"#;
    let findings = scan_php_unsafe_patterns("app/Helpers/legacy.php", source);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_PHP_CREATE_FUNCTION);
}

#[test]
fn test_php_comments_and_safe_code_ignored() {
    let source = r#"<?php
// eval("hello");
# unserialize($test);
/*
exec("whoami");
create_function('$a', 'return $a;');
*/
$obj = new CustomEvaluator();
$obj->evaluate($item);
$str = "This is a safe evaluation function";
"#;
    let findings = scan_php_unsafe_patterns("app/Services/SafeService.php", source);
    assert!(
        findings.is_empty(),
        "Expected 0 findings in safe PHP source"
    );
}
