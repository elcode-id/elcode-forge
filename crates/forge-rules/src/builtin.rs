use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_core::{Confidence, Finding, Module, Repository, Severity, SourceLocation};

/// Rule ID: Disallows UI/component/page modules from directly importing database modules.
pub const RULE_NO_DIRECT_DB_IN_UI: &str = "arch/no-direct-db-in-ui";

/// Rule ID: Disallows circular dependencies between repository modules.
pub const RULE_NO_CIRCULAR_DEPS: &str = "arch/no-circular-dependencies";

/// Rule ID: Requires service modules to have corresponding unit/integration tests.
pub const RULE_REQUIRE_SERVICE_TESTS: &str = "quality/require-service-tests";

/// Rule ID: Enforces a maximum line count per source file.
pub const RULE_MAX_FILE_LINES: &str = "quality/max-file-lines";

/// Evaluates `arch/no-direct-db-in-ui` on `repo`.
pub fn check_no_direct_db_in_ui(repo: &Repository, severity: Severity) -> Vec<Finding> {
    let mut findings = Vec::new();

    for module in &repo.modules {
        if !is_ui_module(module) {
            continue;
        }

        for imported in &module.imports {
            if is_db_module_id(imported) {
                let first_file = module.files.first().cloned();
                findings.push(Finding {
                    id: format!("ARCH-UI-DB-{}-{}", module.id, imported),
                    category: "architecture".to_string(),
                    severity,
                    confidence: Confidence::Certain,
                    title: format!(
                        "UI module '{}' directly imports database layer '{}'",
                        module.id, imported
                    ),
                    description: format!(
                        "Module '{}' accesses database module '{}' directly, bypassing the service boundary.",
                        module.id, imported
                    ),
                    evidence: format!("{} -> {}", module.id, imported),
                    location: first_file.map(|file_path| SourceLocation {
                        file_path,
                        start_line: None,
                        end_line: None,
                    }),
                    recommendation:
                        "Route database queries through an application service or repository layer."
                            .to_string(),
                    rule_id: RULE_NO_DIRECT_DB_IN_UI.to_string(),
                });
            }
        }
    }

    findings
}

/// Evaluates `arch/no-circular-dependencies` on `repo`.
pub fn check_no_circular_dependencies(repo: &Repository, severity: Severity) -> Vec<Finding> {
    let cycles = find_module_cycles(&repo.modules);
    let mut findings = Vec::new();

    for cycle in cycles {
        let joined = cycle.join(" <-> ");
        let location = repo
            .modules
            .iter()
            .find(|m| Some(&m.id) == cycle.first())
            .and_then(|m| m.files.first().cloned())
            .map(|file_path| SourceLocation {
                file_path,
                start_line: None,
                end_line: None,
            });

        findings.push(Finding {
            id: format!("ARCH-CYCLE-{}", cycle.join("_")),
            category: "architecture".to_string(),
            severity,
            confidence: Confidence::Certain,
            title: format!("Circular dependency detected across modules: {joined}"),
            description: format!(
                "Modules [{}] form a strongly connected import cycle, increasing coupling and change risk.",
                cycle.join(", ")
            ),
            evidence: joined,
            location,
            recommendation:
                "Extract shared contracts/types into a lower-level module to break the import cycle."
                    .to_string(),
            rule_id: RULE_NO_CIRCULAR_DEPS.to_string(),
        });
    }

    findings
}

/// Evaluates `quality/require-service-tests` on `repo`.
pub fn check_require_service_tests(repo: &Repository, severity: Severity) -> Vec<Finding> {
    let test_files: Vec<&str> = repo
        .files
        .iter()
        .filter(|f| f.is_test)
        .map(|f| f.relative_path.as_str())
        .collect();

    let mut findings = Vec::new();

    for module in &repo.modules {
        if !is_service_module(module) {
            continue;
        }

        for file_path in &module.files {
            if is_test_like(file_path) {
                continue;
            }
            let Some(stem) = Path::new(file_path).file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let stem_lower = stem.to_ascii_lowercase();
            let has_test = test_files
                .iter()
                .any(|t| t.to_ascii_lowercase().contains(&stem_lower));

            if !has_test {
                findings.push(Finding {
                    id: format!("QUAL-SVC-TEST-{file_path}"),
                    category: "testing".to_string(),
                    severity,
                    confidence: Confidence::High,
                    title: format!("Service file '{file_path}' has no detected test suite"),
                    description: format!(
                        "Critical service module file '{file_path}' lacks a corresponding unit or integration test."
                    ),
                    evidence: format!("No test file matching '{stem}.test.*' or '{stem}.spec.*'"),
                    location: Some(SourceLocation {
                        file_path: file_path.clone(),
                        start_line: None,
                        end_line: None,
                    }),
                    recommendation: format!(
                        "Add a unit or integration test covering '{file_path}' (e.g., '{stem}.test.ts')."
                    ),
                    rule_id: RULE_REQUIRE_SERVICE_TESTS.to_string(),
                });
            }
        }
    }

    findings
}

/// Evaluates `quality/max-file-lines` on `repo`.
pub fn check_max_file_lines(repo: &Repository, severity: Severity, threshold: u32) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in &repo.files {
        let is_code_file = matches!(
            file.language.as_deref(),
            Some("typescript" | "javascript" | "rust" | "python" | "go")
        );
        if is_code_file && file.line_count > threshold {
            findings.push(Finding {
                id: format!("QUAL-MAX-LINES-{}", file.relative_path),
                category: "maintainability".to_string(),
                severity,
                confidence: Confidence::Certain,
                title: format!(
                    "File '{}' exceeds maximum line threshold ({} > {})",
                    file.relative_path, file.line_count, threshold
                ),
                description: format!(
                    "Source file '{}' has {} lines, exceeding the configured threshold of {} lines.",
                    file.relative_path, file.line_count, threshold
                ),
                evidence: format!("line_count = {}", file.line_count),
                location: Some(SourceLocation {
                    file_path: file.relative_path.clone(),
                    start_line: Some(1),
                    end_line: Some(file.line_count),
                }),
                recommendation:
                    "Split large files into smaller, focused modules with single responsibilities."
                        .to_string(),
                rule_id: RULE_MAX_FILE_LINES.to_string(),
            });
        }
    }

    findings
}

fn is_ui_module(module: &Module) -> bool {
    let name = module.name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "components" | "pages" | "views" | "ui" | "routes"
    )
}

fn is_db_module_id(module_id: &str) -> bool {
    let lower = module_id.to_ascii_lowercase();
    lower == "db"
        || lower == "database"
        || lower == "prisma"
        || lower == "drizzle"
        || lower.ends_with("/db")
        || lower.ends_with("/database")
        || lower.ends_with("/prisma")
        || lower.ends_with("/drizzle")
}

fn is_service_module(module: &Module) -> bool {
    let lower = module.name.to_ascii_lowercase();
    lower == "services" || lower == "service"
}

fn is_test_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains(".test.") || lower.contains(".spec.") || lower.contains("_test.")
}

fn find_module_cycles(modules: &[Module]) -> Vec<Vec<String>> {
    struct Tarjan<'a> {
        adj: BTreeMap<&'a str, Vec<&'a str>>,
        index: usize,
        indices: BTreeMap<&'a str, usize>,
        lowlink: BTreeMap<&'a str, usize>,
        stack: Vec<&'a str>,
        on_stack: BTreeSet<&'a str>,
        cycles: Vec<Vec<String>>,
    }

    fn visit<'a>(v: &'a str, t: &mut Tarjan<'a>) {
        let idx = t.index;
        t.indices.insert(v, idx);
        t.lowlink.insert(v, idx);
        t.index += 1;
        t.stack.push(v);
        t.on_stack.insert(v);

        let neighbors = t.adj.get(v).cloned().unwrap_or_default();
        for w in neighbors {
            if !t.indices.contains_key(w) {
                visit(w, t);
                let low_v = t.lowlink.get(v).copied().unwrap_or(0);
                let low_w = t.lowlink.get(w).copied().unwrap_or(0);
                t.lowlink.insert(v, low_v.min(low_w));
            } else if t.on_stack.contains(w) {
                let low_v = t.lowlink.get(v).copied().unwrap_or(0);
                let idx_w = t.indices.get(w).copied().unwrap_or(0);
                t.lowlink.insert(v, low_v.min(idx_w));
            }
        }

        if t.lowlink.get(v) == t.indices.get(v) {
            let mut scc = Vec::new();
            while let Some(w) = t.stack.pop() {
                t.on_stack.remove(w);
                scc.push(w.to_string());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                scc.sort();
                t.cycles.push(scc);
            }
        }
    }

    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for m in modules {
        adj.insert(
            m.id.as_str(),
            m.imports.iter().map(String::as_str).collect(),
        );
    }

    let mut state = Tarjan {
        adj,
        index: 0,
        indices: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        cycles: Vec::new(),
    };

    for m in modules {
        if !state.indices.contains_key(m.id.as_str()) {
            visit(m.id.as_str(), &mut state);
        }
    }

    state.cycles.sort();
    state.cycles
}
