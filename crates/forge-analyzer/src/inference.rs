//! Graph-theoretic topology and boundary inference engine (`eforge init --infer` / `eforge rules infer`).
//!
//! Uses module Fan-In ($Ca$), Fan-Out ($Ce$), Robert C. Martin Instability ($I = Ce / (Ca + Ce)$),
//! and directory clustering to classify architectural layers, detect the repository archetype,
//! and synthesize custom declarative boundary rules.

use std::collections::BTreeMap;

use forge_core::{FileNode, Module};
use serde::{Deserialize, Serialize};

use crate::graph::ArchitectureGraph;

/// Classified architectural layer role for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitecturalLayer {
    Foundation,
    Domain,
    Application,
    Infrastructure,
    Presentation,
    Utility,
    Tests,
}

impl ArchitecturalLayer {
    /// Returns the human-readable snake_case identifier for the layer.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Domain => "domain",
            Self::Application => "application",
            Self::Infrastructure => "infrastructure",
            Self::Presentation => "presentation",
            Self::Utility => "utility",
            Self::Tests => "tests",
        }
    }
}

/// High-level architectural archetype detected from module topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitecturalArchetype {
    LayeredArchitecture,
    CleanHexagonal,
    WorkspaceDag,
    ModularMonolith,
    FlatService,
}

impl ArchitecturalArchetype {
    /// Returns the canonical display label for the archetype.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LayeredArchitecture => "Layered Architecture (Presentation -> Service -> Data)",
            Self::CleanHexagonal => {
                "Clean / Hexagonal Architecture (Adapters -> Application -> Domain)"
            }
            Self::WorkspaceDag => {
                "Multi-Package Workspace DAG (Leaf Core -> Domain Crates -> CLI/Apps)"
            }
            Self::ModularMonolith => "Modular Monolith (Feature-Sliced Modules)",
            Self::FlatService => "Standard Modular Service",
        }
    }
}

/// Graph-theoretic centrality and instability metrics for a single module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleTopologyMetrics {
    pub module_id: String,
    pub afferent_coupling_ca: usize,
    pub efferent_coupling_ce: usize,
    pub instability_index: f64,
    pub inferred_layer: ArchitecturalLayer,
    pub file_count: usize,
}

/// A custom declarative rule synthesized from repository topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferredBoundaryRule {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: String,
    pub matcher_type: String,
    pub source_glob: String,
    pub forbidden_target: Option<String>,
    pub companion_pattern: Option<String>,
    pub description: String,
    pub remediation: String,
    pub confidence_pct: u8,
    pub current_violations: usize,
}

/// Complete topology inference report produced by [`infer_topology_and_boundaries`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyInferenceReport {
    pub archetype: ArchitecturalArchetype,
    pub archetype_label: String,
    pub module_metrics: Vec<ModuleTopologyMetrics>,
    pub inferred_rules: Vec<InferredBoundaryRule>,
    pub recommended_packs: Vec<String>,
    pub generated_rules_yaml: String,
}

/// Analyzes `graph` and `files` to classify module roles, detect the architectural archetype,
/// and synthesize high-confidence declarative boundary rules.
#[must_use]
pub fn infer_topology_and_boundaries(
    files: &[FileNode],
    graph: &ArchitectureGraph,
) -> TopologyInferenceReport {
    let mut ca_map: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ce_map: BTreeMap<&str, usize> = BTreeMap::new();

    for m in &graph.modules {
        ca_map.entry(m.id.as_str()).or_insert(0);
        ce_map.insert(m.id.as_str(), m.imports.len());
    }
    for m in &graph.modules {
        for imported in &m.imports {
            if let Some(count) = ca_map.get_mut(imported.as_str()) {
                *count += 1;
            }
        }
    }

    let mut module_metrics = Vec::with_capacity(graph.modules.len());
    let mut layer_modules: BTreeMap<ArchitecturalLayer, Vec<&Module>> = BTreeMap::new();

    for m in &graph.modules {
        let ca = ca_map.get(m.id.as_str()).copied().unwrap_or(0);
        let ce = ce_map.get(m.id.as_str()).copied().unwrap_or(0);
        let total = ca + ce;
        let instability = if total == 0 {
            0.5
        } else {
            ((ce as f64 / total as f64) * 100.0).round() / 100.0
        };
        let layer = classify_module_layer(&m.id, &m.name, ca, ce);
        layer_modules.entry(layer).or_default().push(m);

        module_metrics.push(ModuleTopologyMetrics {
            module_id: m.id.clone(),
            afferent_coupling_ca: ca,
            efferent_coupling_ce: ce,
            instability_index: instability,
            inferred_layer: layer,
            file_count: m.files.len(),
        });
    }

    // Sort metrics by afferent coupling descending (most foundational first), then ID
    module_metrics.sort_by(|a, b| {
        b.afferent_coupling_ca
            .cmp(&a.afferent_coupling_ca)
            .then_with(|| a.module_id.cmp(&b.module_id))
    });

    let archetype = detect_archetype(&graph.modules, &layer_modules);
    let mut inferred_rules = Vec::new();

    // 1. Workspace Foundation Leaf Rule (e.g. forge-core or core/shared module)
    if let Some(foundation_mods) = layer_modules.get(&ArchitecturalLayer::Foundation) {
        for fmod in foundation_mods {
            let siblings: Vec<String> = graph
                .modules
                .iter()
                .filter(|other| other.id != fmod.id)
                .map(|other| other.id.clone())
                .collect();
            if !siblings.is_empty() {
                let slug = sanitize_slug(&fmod.name);
                let violations = fmod.imports.len();
                inferred_rules.push(InferredBoundaryRule {
                    id: format!("arch/leaf-isolation-{slug}"),
                    name: format!("Leaf Isolation for '{}'", fmod.id),
                    category: "architecture".to_string(),
                    severity: "high".to_string(),
                    matcher_type: "forbidden_imports".to_string(),
                    source_glob: format!("{}/**", fmod.root_path),
                    forbidden_target: Some(siblings.first().cloned().unwrap_or_default()),
                    companion_pattern: None,
                    description: format!(
                        "Foundational module '{}' (Ca={}, Ce={}) must remain a pure leaf without importing higher-level modules.",
                        fmod.id,
                        ca_map.get(fmod.id.as_str()).copied().unwrap_or(0),
                        fmod.imports.len()
                    ),
                    remediation: format!(
                        "Move shared traits or models into '{}' and invert dependencies from upper layers.",
                        fmod.id
                    ),
                    confidence_pct: if violations == 0 { 98 } else { 85 },
                    current_violations: violations,
                });
            }
        }
    }

    // 2. Presentation -> Infrastructure Bypass Rule
    if let (Some(pres_mods), Some(infra_mods)) = (
        layer_modules.get(&ArchitecturalLayer::Presentation),
        layer_modules.get(&ArchitecturalLayer::Infrastructure),
    ) {
        for pmod in pres_mods {
            for imod in infra_mods {
                let violations = usize::from(pmod.imports.contains(&imod.id));
                let p_slug = sanitize_slug(&pmod.name);
                let i_slug = sanitize_slug(&imod.name);
                inferred_rules.push(InferredBoundaryRule {
                    id: format!("arch/no-{p_slug}-to-{i_slug}"),
                    name: format!("No Direct '{}' Import in '{}'", imod.name, pmod.name),
                    category: "architecture".to_string(),
                    severity: "high".to_string(),
                    matcher_type: "forbidden_imports".to_string(),
                    source_glob: format!("{}/**", pmod.root_path),
                    forbidden_target: Some(imod.root_path.clone()),
                    companion_pattern: None,
                    description: format!(
                        "Presentation module '{}' must not import infrastructure module '{}' directly.",
                        pmod.id, imod.id
                    ),
                    remediation: "Route data access through an application or service layer module."
                        .to_string(),
                    confidence_pct: 95,
                    current_violations: violations,
                });
            }
        }
    }

    // 3. Domain Purity Rule (Domain must not import Presentation or Infrastructure)
    if let Some(domain_mods) = layer_modules.get(&ArchitecturalLayer::Domain) {
        let mut forbidden_targets = Vec::new();
        if let Some(pres) = layer_modules.get(&ArchitecturalLayer::Presentation) {
            for p in pres {
                forbidden_targets.push(p.root_path.clone());
            }
        }
        if let Some(infra) = layer_modules.get(&ArchitecturalLayer::Infrastructure) {
            for i in infra {
                forbidden_targets.push(i.root_path.clone());
            }
        }
        if let (Some(first_dom), Some(first_forb)) =
            (domain_mods.first(), forbidden_targets.first())
        {
            let slug = sanitize_slug(&first_dom.name);
            let violations = first_dom
                .imports
                .iter()
                .filter(|imp| forbidden_targets.contains(imp))
                .count();
            inferred_rules.push(InferredBoundaryRule {
                id: format!("arch/domain-purity-{slug}"),
                name: format!("Domain Purity for '{}'", first_dom.id),
                category: "architecture".to_string(),
                severity: "high".to_string(),
                matcher_type: "forbidden_imports".to_string(),
                source_glob: format!("{}/**", first_dom.root_path),
                forbidden_target: Some(first_forb.clone()),
                companion_pattern: None,
                description: format!(
                    "Domain module '{}' must remain independent of UI and outer adapter layers.",
                    first_dom.id
                ),
                remediation:
                    "Depend on domain interfaces/traits rather than concrete outer adapters."
                        .to_string(),
                confidence_pct: 92,
                current_violations: violations,
            });
        }
    }

    // 4. Service / Application Companion Test Rule
    if let Some(app_mods) = layer_modules.get(&ArchitecturalLayer::Application) {
        if let Some(first_app) = app_mods.first() {
            let companion = infer_test_companion_pattern(files, &first_app.root_path);
            let slug = sanitize_slug(&first_app.name);
            inferred_rules.push(InferredBoundaryRule {
                id: format!("quality/require-{slug}-tests"),
                name: format!("Require Companion Tests in '{}'", first_app.id),
                category: "quality".to_string(),
                severity: "medium".to_string(),
                matcher_type: "required_companion".to_string(),
                source_glob: format!("{}/**", first_app.root_path),
                forbidden_target: None,
                companion_pattern: Some(companion),
                description: format!(
                    "Application service files in '{}' should maintain companion unit tests.",
                    first_app.id
                ),
                remediation: "Create a companion test file covering service behavior.".to_string(),
                confidence_pct: 88,
                current_violations: 0,
            });
        }
    }

    let mut recommended_packs = vec!["recommended".to_string()];
    if matches!(archetype, ArchitecturalArchetype::CleanHexagonal) {
        recommended_packs.push("clean-architecture".to_string());
    }
    if graph.circular_dependencies.is_empty() {
        recommended_packs.push("strict".to_string());
    }

    let generated_rules_yaml = render_inferred_rules_yaml(archetype, &inferred_rules);

    TopologyInferenceReport {
        archetype,
        archetype_label: archetype.as_str().to_string(),
        module_metrics,
        inferred_rules,
        recommended_packs,
        generated_rules_yaml,
    }
}

fn classify_module_layer(mod_id: &str, mod_name: &str, ca: usize, ce: usize) -> ArchitecturalLayer {
    let lower_id = mod_id.to_ascii_lowercase();
    let lower_name = mod_name.to_ascii_lowercase();

    if lower_id.contains("test") || lower_name == "tests" || lower_name == "__tests__" {
        return ArchitecturalLayer::Tests;
    }
    if lower_name.ends_with("-core")
        || lower_name == "core"
        || lower_name == "shared"
        || lower_name == "common"
        || lower_name == "types"
        || lower_name == "schema"
        || (ca >= 2 && ce == 0)
    {
        return ArchitecturalLayer::Foundation;
    }
    if lower_name == "domain"
        || lower_name == "entities"
        || lower_name == "models"
        || lower_id.contains("/domain")
    {
        return ArchitecturalLayer::Domain;
    }
    if lower_name == "db"
        || lower_name == "database"
        || lower_name == "infra"
        || lower_name == "infrastructure"
        || lower_name == "repositories"
        || lower_name == "prisma"
        || lower_name == "adapters"
    {
        return ArchitecturalLayer::Infrastructure;
    }
    if lower_name == "pages"
        || lower_name == "components"
        || lower_name == "routes"
        || lower_name == "ui"
        || lower_name == "views"
        || lower_name == "controllers"
        || lower_name == "handlers"
        || lower_name.ends_with("-cli")
        || lower_name == "cli"
        || lower_id.starts_with("cmd/")
    {
        return ArchitecturalLayer::Presentation;
    }
    if lower_name == "utils" || lower_name == "helpers" || lower_name == "lib" {
        return ArchitecturalLayer::Utility;
    }
    ArchitecturalLayer::Application
}

fn detect_archetype(
    modules: &[Module],
    layer_modules: &BTreeMap<ArchitecturalLayer, Vec<&Module>>,
) -> ArchitecturalArchetype {
    if modules
        .iter()
        .any(|m| m.id.starts_with("crates/") || m.id.starts_with("packages/"))
    {
        return ArchitecturalArchetype::WorkspaceDag;
    }
    if layer_modules.contains_key(&ArchitecturalLayer::Domain)
        && (layer_modules.contains_key(&ArchitecturalLayer::Infrastructure)
            || layer_modules.contains_key(&ArchitecturalLayer::Application))
    {
        return ArchitecturalArchetype::CleanHexagonal;
    }
    if layer_modules.contains_key(&ArchitecturalLayer::Presentation)
        && (layer_modules.contains_key(&ArchitecturalLayer::Application)
            || layer_modules.contains_key(&ArchitecturalLayer::Infrastructure))
    {
        return ArchitecturalArchetype::LayeredArchitecture;
    }
    if modules.len() >= 4 {
        return ArchitecturalArchetype::ModularMonolith;
    }
    ArchitecturalArchetype::FlatService
}

fn infer_test_companion_pattern(files: &[FileNode], root_path: &str) -> String {
    let has_spec = files.iter().any(|f| f.relative_path.contains(".spec."));
    let has_py = files.iter().any(|f| f.relative_path.ends_with(".py"));
    let has_go = files.iter().any(|f| f.relative_path.ends_with(".go"));
    let has_rs = files.iter().any(|f| f.relative_path.ends_with(".rs"));

    if has_rs {
        format!("{root_path}/tests/{{name}}_test.rs")
    } else if has_go {
        format!("{root_path}/{{name}}_test.go")
    } else if has_py {
        "tests/test_{name}.py".to_string()
    } else if has_spec {
        format!("{root_path}/{{name}}.spec.ts")
    } else {
        format!("{root_path}/{{name}}.test.ts")
    }
}

fn sanitize_slug(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn render_inferred_rules_yaml(
    archetype: ArchitecturalArchetype,
    rules: &[InferredBoundaryRule],
) -> String {
    let mut out = String::new();
    out.push_str("# GENERATED BY ELCODE FORGE (eforge rules infer)\n");
    out.push_str(&format!("# Detected Archetype: {}\n", archetype.as_str()));
    out.push_str("version: 1\n");
    out.push_str("pack: inferred-architecture\n");
    out.push_str(
        "description: \"Graph-theoretically inferred architectural boundaries and companion rules\"\n",
    );
    if rules.is_empty() {
        out.push_str("rules: []\n");
        return out;
    }
    out.push_str("rules:\n");
    for r in rules {
        out.push_str(&format!("  - id: \"{}\"\n", r.id));
        out.push_str(&format!("    name: \"{}\"\n", r.name));
        out.push_str(&format!("    category: {}\n", r.category));
        out.push_str(&format!("    severity: {}\n", r.severity));
        out.push_str(&format!("    description: \"{}\"\n", r.description));
        out.push_str("    match:\n");
        out.push_str(&format!("      type: {}\n", r.matcher_type));
        out.push_str(&format!("      files: [\"{}\"]\n", r.source_glob));
        if let Some(forb) = &r.forbidden_target {
            out.push_str(&format!("      forbidden: [\"{}\"]\n", forb));
        }
        if let Some(comp) = &r.companion_pattern {
            out.push_str(&format!("      companion: \"{}\"\n", comp));
        }
        out.push_str(&format!("    remediation: \"{}\"\n", r.remediation));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::build_ts_architecture_graph;
    use crate::scanner::scan_repository;
    use std::path::PathBuf;

    #[test]
    fn infers_layered_architecture_and_boundary_rules_on_next_app() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let discovery = scan_repository(&fixture, &[]);
        assert!(discovery.is_ok());
        let Ok(summary) = discovery else { return };

        let graph = build_ts_architecture_graph(&fixture, &summary.files);
        assert!(graph.is_ok());
        let Ok(g) = graph else { return };

        let report = infer_topology_and_boundaries(&summary.files, &g);
        assert_eq!(
            report.archetype,
            ArchitecturalArchetype::LayeredArchitecture
        );
        assert!(!report.module_metrics.is_empty());
        assert!(!report.inferred_rules.is_empty());
        assert!(report
            .generated_rules_yaml
            .contains("pack: inferred-architecture"));
        assert!(report
            .inferred_rules
            .iter()
            .any(|r| r.id.starts_with("arch/no-")));
    }
}
