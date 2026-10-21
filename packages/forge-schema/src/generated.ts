/* eslint-disable */
/**
 * This file was automatically generated from report-bundle.schema.json.
 * DO NOT MODIFY IT BY HAND. Instead, modify the Rust IR structs and run `eforge schema`.
 */

/**
 * Categorical risk level for a pull request or working-tree change set.
 */
export type RiskLevel = "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
/**
 * Normalized confidence levels ordered from lowest (`Low`) to highest (`Certain`).
 */
export type Confidence = "low" | "medium" | "high" | "certain";
/**
 * Normalized finding severity levels ordered from lowest (`Info`) to highest (`Critical`).
 */
export type Severity = "info" | "low" | "medium" | "high" | "critical";

/**
 * Versioned, deterministic intelligence bundle consumable by the web dashboard.
 */
export interface ReportBundle {
  architecture: ArchitectureSummary;
  change_impact?: ChangeImpactReport | null;
  change_risk?: RiskReport | null;
  hotspots: HotspotEntry[];
  repository: Repository;
  revision?: GitRevision | null;
  rules: RuleDescriptor[];
  schema_version: number;
  tool_version: string;
  [k: string]: unknown;
}
/**
 * High-level architecture summary for the dashboard.
 */
export interface ArchitectureSummary {
  circular_dependencies: string[][];
  high_centrality_modules: string[];
  [k: string]: unknown;
}
/**
 * Blast-radius change impact report for a Git diff.
 */
export interface ChangeImpactReport {
  changed_files: FileChange[];
  changed_files_without_tests: string[];
  directly_affected_modules: string[];
  transitively_affected_modules: string[];
  [k: string]: unknown;
}
/**
 * Normalized file-level change extracted from a unified Git diff.
 */
export interface FileChange {
  added_lines: number;
  deleted_lines: number;
  is_deleted: boolean;
  is_new: boolean;
  path: string;
  [k: string]: unknown;
}
/**
 * Explainable change risk assessment (distinct from overall project health).
 */
export interface RiskReport {
  level: RiskLevel;
  reasons: string[];
  score: number;
  [k: string]: unknown;
}
/**
 * File-level churn hotspot summary computed from Git commit history.
 */
export interface HotspotEntry {
  commit_count: number;
  contributors_count: number;
  path: string;
  [k: string]: unknown;
}
/**
 * Language-neutral Intermediate Representation (IR) of an analyzed repository.
 */
export interface Repository {
  dependencies: Dependency[];
  detected_frameworks: string[];
  detected_languages: string[];
  files: FileNode[];
  findings: Finding[];
  health?: HealthScore | null;
  modules: Module[];
  name: string;
  package_managers: string[];
  root_path: string;
  [k: string]: unknown;
}
/**
 * External package dependency metadata.
 */
export interface Dependency {
  ecosystem: string;
  is_dev: boolean;
  is_direct: boolean;
  name: string;
  resolved_version?: string | null;
  version_req: string;
  [k: string]: unknown;
}
/**
 * Normalized file metadata node in the repository graph.
 */
export interface FileNode {
  content_hash: string;
  is_test: boolean;
  language?: string | null;
  line_count: number;
  relative_path: string;
  size_bytes: number;
  [k: string]: unknown;
}
/**
 * Normalized finding emitted by deterministic analyzers, rules, or security scanners.
 */
export interface Finding {
  category: string;
  confidence: Confidence;
  description: string;
  evidence: string;
  id: string;
  location?: SourceLocation | null;
  recommendation: string;
  rule_id: string;
  severity: Severity;
  title: string;
  [k: string]: unknown;
}
/**
 * Source code location for a finding or symbol within the repository.
 */
export interface SourceLocation {
  end_line?: number | null;
  file_path: string;
  start_line?: number | null;
  [k: string]: unknown;
}
/**
 * Transparent overall codebase health score (`0..=100`) and breakdown.
 */
export interface HealthScore {
  dimensions: DimensionScore[];
  overall: number;
  [k: string]: unknown;
}
/**
 * Explainable health score for an individual engineering dimension.
 */
export interface DimensionScore {
  dimension: string;
  reasons: string[];
  score: number;
  weight_pct: number;
  [k: string]: unknown;
}
/**
 * Logical module boundary inferred from repository structure and import relationships.
 */
export interface Module {
  exports: string[];
  files: string[];
  id: string;
  imports: string[];
  name: string;
  root_path: string;
  [k: string]: unknown;
}
/**
 * Git HEAD revision metadata.
 */
export interface GitRevision {
  commit_unix_ts: number;
  head_sha: string;
  [k: string]: unknown;
}
/**
 * Rule metadata embedded in [`ReportBundle`] for rules catalog and violation linkage.
 */
export interface RuleDescriptor {
  category: string;
  description: string;
  enabled: boolean;
  id: string;
  name: string;
  remediation?: string | null;
  severity: Severity;
  source: string;
  [k: string]: unknown;
}
