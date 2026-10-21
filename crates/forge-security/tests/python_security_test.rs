use std::error::Error;

use forge_core::Severity;
use forge_security::{
    scan_python_unsafe_patterns, RULE_PYTHON_INSECURE_SUBPROCESS,
    RULE_PYTHON_PICKLE_DESERIALIZATION, RULE_PYTHON_UNSAFE_EVAL,
};

#[test]
fn test_python_unsafe_eval_and_exec_detected() -> Result<(), Box<dyn Error>> {
    let code = r#"
# Safe function
def safe_evaluator():
    evaluate_score = 42
    return evaluate_score

# Unsafe calls
result = eval(user_input)
exec(dynamic_code_string)
"#;

    let findings = scan_python_unsafe_patterns("app/services/eval_service.py", code);
    let eval_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == RULE_PYTHON_UNSAFE_EVAL)
        .collect();

    assert_eq!(eval_findings.len(), 2);
    assert_eq!(eval_findings[0].severity, Severity::High);
    assert!(eval_findings[0].evidence.contains("eval(user_input)"));
    assert!(eval_findings[1]
        .evidence
        .contains("exec(dynamic_code_string)"));

    Ok(())
}

#[test]
fn test_python_insecure_subprocess_and_os_system_detected() -> Result<(), Box<dyn Error>> {
    let code = r#"
import os
import subprocess

# Safe subprocess execution
subprocess.run(["ls", "-la"], shell=False)

# Insecure subprocess executions
subprocess.run(f"echo {user_input}", shell=True)
p = subprocess.Popen("cat /etc/passwd", shell = True)

# Insecure legacy os calls
os.system(user_command)
pipe = os.popen("whoami")
"#;

    let findings = scan_python_unsafe_patterns("app/services/process_service.py", code);
    let subprocess_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == RULE_PYTHON_INSECURE_SUBPROCESS)
        .collect();

    assert_eq!(subprocess_findings.len(), 4);
    for f in &subprocess_findings {
        assert_eq!(f.severity, Severity::High);
    }

    assert!(subprocess_findings[0]
        .evidence
        .contains("subprocess.run(f\"echo {user_input}\", shell=True)"));
    assert!(subprocess_findings[1]
        .evidence
        .contains("subprocess.Popen(\"cat /etc/passwd\", shell = True)"));
    assert!(subprocess_findings[2]
        .evidence
        .contains("os.system(user_command)"));
    assert!(subprocess_findings[3]
        .evidence
        .contains("pipe = os.popen(\"whoami\")"));

    Ok(())
}

#[test]
fn test_python_pickle_deserialization_detected() -> Result<(), Box<dyn Error>> {
    let code = r#"
import pickle
import _pickle

# Safe serialization
payload = pickle.dumps({"user": "admin"})

# Unsafe deserializations
obj1 = pickle.loads(raw_bytes)
obj2 = _pickle.loads(untrusted_stream)
obj3 = pickle.load(fp)
obj4 = _pickle.load(sock)
"#;

    let findings = scan_python_unsafe_patterns("app/services/cache_service.py", code);
    let pickle_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == RULE_PYTHON_PICKLE_DESERIALIZATION)
        .collect();

    assert_eq!(pickle_findings.len(), 4);
    for f in &pickle_findings {
        assert_eq!(f.severity, Severity::High);
    }

    assert!(pickle_findings[0]
        .evidence
        .contains("obj1 = pickle.loads(raw_bytes)"));
    assert!(pickle_findings[1]
        .evidence
        .contains("obj2 = _pickle.loads(untrusted_stream)"));
    assert!(pickle_findings[2]
        .evidence
        .contains("obj3 = pickle.load(fp)"));
    assert!(pickle_findings[3]
        .evidence
        .contains("obj4 = _pickle.load(sock)"));

    Ok(())
}

#[test]
fn test_python_comments_and_docstrings_ignored() -> Result<(), Box<dyn Error>> {
    let code = r#"
# eval("never executed")
# os.system("rm -rf /")
# pickle.loads(b"data")

"""
Documentation explaining why eval(x) is dangerous.
Also os.system(cmd) and pickle.loads(payload) should not be used.
"""

'''
Single quote docstring discussing eval(val) and subprocess shell=True.
'''

def real_safe_func():
    return 100
"#;

    let findings = scan_python_unsafe_patterns("app/services/docs.py", code);
    assert!(
        findings.is_empty(),
        "Expected no findings in commented code"
    );

    Ok(())
}

#[test]
fn test_non_python_files_ignored() -> Result<(), Box<dyn Error>> {
    let ts_code = "eval(userInput);";
    let findings = scan_python_unsafe_patterns("src/app.ts", ts_code);
    assert!(findings.is_empty());

    Ok(())
}
