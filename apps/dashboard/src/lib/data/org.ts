/**
 * Enterprise Organization, Portfolio Rollups & Policy Governance Data Module.
 * Connects with `forge-server` organization and audit endpoints, and provides client-side policy evaluation.
 */

import type { ReportBundle, Finding } from "@elcode-forge/schema";

export interface OrgListItem {
  id: string;
  name: string;
  slug: string;
  health_score: number;
  total_projects: number;
  total_vulnerabilities: number;
  created_at: number;
  updated_at: number;
}

export interface CrossRepoVulnerability {
  rule_id: string;
  severity: string;
  title: string;
  category: string;
  affected_projects: string[];
  occurrence_count: number;
}

export interface OrganizationSummary {
  id: string;
  name: string;
  slug: string;
  health_score: number;
  total_projects: number;
  total_vulnerabilities: number;
  cross_repo_vulnerabilities: CrossRepoVulnerability[];
  project_ids: string[];
  updated_at: number;
}

export interface AuditEvent {
  id: string;
  org_id: string;
  project_id: string | null;
  actor: string;
  action: string;
  resource_type: string;
  resource_id: string;
  details_json: string | null;
  created_at: number;
}

export interface AuditEventFilter {
  org_id?: string;
  project_id?: string;
  actor?: string;
  action?: string;
  since?: number;
  until?: number;
  limit?: number;
}

export interface PolicyWaiver {
  rule_id: string;
  path_pattern?: string;
  expires_at?: string;
  justification: string;
}

export interface PolicyRuleOverride {
  rule_id: string;
  severity?: string;
  enabled: boolean;
}

export interface PolicyConstraints {
  min_health_score?: number;
  disallowed_severities?: string[];
  forbidden_licenses?: string[];
  max_circular_dependencies?: number;
  max_blast_radius_modules?: number;
  enforce_secret_zero_tolerance?: boolean;
}

export interface PolicyDocument {
  name: string;
  version: string;
  description?: string;
  constraints: PolicyConstraints;
  waivers: PolicyWaiver[];
  overrides: PolicyRuleOverride[];
}

export interface PolicyEvaluationResult {
  verdict: "passed" | "warning" | "violation";
  passed_checks: string[];
  warnings: string[];
  violations: string[];
  active_waivers: PolicyWaiver[];
  expired_waivers: PolicyWaiver[];
}

export const STARTER_POLICY_YAML = `name: enterprise-standard-gate
version: 1.0.0
description: Organization security, license compliance, and architectural boundary constraints.

constraints:
  min_health_score: 80
  enforce_secret_zero_tolerance: true
  disallowed_severities:
    - critical
    - high
  forbidden_licenses:
    - GPL-3.0-only
    - AGPL-3.0-only
  max_circular_dependencies: 0
  max_blast_radius_modules: 5

waivers:
  - rule_id: security/secret-generic-token
    path_pattern: "**/fixtures/**"
    expires_at: "2027-01-01T00:00:00Z"
    justification: "Synthetic test credentials used exclusively in fixture test suites"

overrides:
  - rule_id: arch/no-circular-dependencies
    severity: high
    enabled: true
`;

/**
 * Parses simple policy YAML text into structured PolicyDocument.
 */
export function parsePolicyYaml(yamlText: string): { ok: true; policy: PolicyDocument } | { ok: false; error: string } {
  try {
    const lines = yamlText.split("\n");
    let name = "unnamed-policy";
    let version = "1.0.0";
    let description = "";
    const constraints: PolicyConstraints = {
      disallowed_severities: [],
      forbidden_licenses: [],
      enforce_secret_zero_tolerance: false,
    };
    const waivers: PolicyWaiver[] = [];
    const overrides: PolicyRuleOverride[] = [];

    let currentSection: "root" | "constraints" | "waivers" | "overrides" = "root";
    let currentConstraintList: "disallowed_severities" | "forbidden_licenses" | null = null;
    let currentWaiver: Partial<PolicyWaiver> | null = null;
    let currentOverride: Partial<PolicyRuleOverride> | null = null;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (!line || line.startsWith("#")) continue;

      if (line.startsWith("name:")) {
        name = line.substring(5).trim().replace(/^["']|["']$/g, "");
      } else if (line.startsWith("version:")) {
        version = line.substring(8).trim().replace(/^["']|["']$/g, "");
      } else if (line.startsWith("description:")) {
        description = line.substring(12).trim().replace(/^["']|["']$/g, "");
      } else if (line.startsWith("constraints:")) {
        currentSection = "constraints";
      } else if (line.startsWith("waivers:")) {
        currentSection = "waivers";
      } else if (line.startsWith("overrides:")) {
        currentSection = "overrides";
      } else if (currentSection === "constraints") {
        if (line.startsWith("disallowed_severities:")) {
          currentConstraintList = "disallowed_severities";
        } else if (line.startsWith("forbidden_licenses:")) {
          currentConstraintList = "forbidden_licenses";
        } else if (line.startsWith("- ") && currentConstraintList === "disallowed_severities") {
          constraints.disallowed_severities?.push(line.substring(2).trim().toLowerCase());
        } else if (line.startsWith("- ") && currentConstraintList === "forbidden_licenses") {
          constraints.forbidden_licenses?.push(line.substring(2).trim().replace(/^["']|["']$/g, ""));
        } else if (line.startsWith("min_health_score:")) {
          currentConstraintList = null;
          constraints.min_health_score = parseInt(line.substring(17).trim(), 10);
        } else if (line.startsWith("enforce_secret_zero_tolerance:")) {
          currentConstraintList = null;
          constraints.enforce_secret_zero_tolerance = line.substring(30).trim().toLowerCase() === "true";
        } else if (line.startsWith("max_circular_dependencies:")) {
          currentConstraintList = null;
          constraints.max_circular_dependencies = parseInt(line.substring(26).trim(), 10);
        } else if (line.startsWith("max_blast_radius_modules:")) {
          currentConstraintList = null;
          constraints.max_blast_radius_modules = parseInt(line.substring(25).trim(), 10);
        }
      } else if (currentSection === "waivers") {
        if (line.startsWith("- rule_id:")) {
          if (currentWaiver && currentWaiver.rule_id) {
            waivers.push(currentWaiver as PolicyWaiver);
          }
          currentWaiver = {
            rule_id: line.substring(10).trim().replace(/^["']|["']$/g, ""),
            justification: "",
          };
        } else if (currentWaiver) {
          if (line.startsWith("path_pattern:")) {
            currentWaiver.path_pattern = line.substring(13).trim().replace(/^["']|["']$/g, "");
          } else if (line.startsWith("expires_at:")) {
            currentWaiver.expires_at = line.substring(11).trim().replace(/^["']|["']$/g, "");
          } else if (line.startsWith("justification:")) {
            currentWaiver.justification = line.substring(14).trim().replace(/^["']|["']$/g, "");
          }
        }
      } else if (currentSection === "overrides") {
        if (line.startsWith("- rule_id:")) {
          if (currentOverride && currentOverride.rule_id) {
            overrides.push(currentOverride as PolicyRuleOverride);
          }
          currentOverride = {
            rule_id: line.substring(10).trim().replace(/^["']|["']$/g, ""),
            enabled: true,
          };
        } else if (currentOverride) {
          if (line.startsWith("severity:")) {
            currentOverride.severity = line.substring(9).trim().toLowerCase();
          } else if (line.startsWith("enabled:")) {
            currentOverride.enabled = line.substring(8).trim().toLowerCase() === "true";
          }
        }
      }
    }

    if (currentWaiver && currentWaiver.rule_id) {
      waivers.push(currentWaiver as PolicyWaiver);
    }
    if (currentOverride && currentOverride.rule_id) {
      overrides.push(currentOverride as PolicyRuleOverride);
    }

    return {
      ok: true,
      policy: {
        name,
        version,
        description,
        constraints,
        waivers,
        overrides,
      },
    };
  } catch (err: unknown) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Checks whether an ISO expiration date has passed.
 */
export function isWaiverExpired(expiresAt?: string): boolean {
  if (!expiresAt) return false;
  try {
    const exp = new Date(expiresAt).getTime();
    return !isNaN(exp) && exp < Date.now();
  } catch {
    return false;
  }
}

/**
 * Evaluates a policy against a ReportBundle and returns passed checks, warnings, and violations.
 */
export function evaluatePolicyAgainstBundle(
  policy: PolicyDocument,
  bundle: ReportBundle | null
): PolicyEvaluationResult {
  const passed_checks: string[] = [];
  const warnings: string[] = [];
  const violations: string[] = [];
  const active_waivers: PolicyWaiver[] = [];
  const expired_waivers: PolicyWaiver[] = [];

  for (const w of policy.waivers) {
    if (isWaiverExpired(w.expires_at)) {
      expired_waivers.push(w);
      warnings.push(`Waiver for '${w.rule_id}' expired on ${w.expires_at}`);
    } else {
      active_waivers.push(w);
    }
  }

  if (!bundle) {
    return {
      verdict: "passed",
      passed_checks: ["Policy syntax and waiver schemas valid (no repository bundle loaded)"],
      warnings,
      violations,
      active_waivers,
      expired_waivers,
    };
  }

  // 1. Min health score check
  const repo = bundle.repository;
  const findings: Finding[] = repo?.findings ?? [];
  const overallHealth = repo?.health?.overall ?? 0;

  if (policy.constraints.min_health_score !== undefined) {
    if (overallHealth >= policy.constraints.min_health_score) {
      passed_checks.push(`Repository health score (${overallHealth}) meets minimum threshold (${policy.constraints.min_health_score})`);
    } else {
      violations.push(`Repository health score (${overallHealth}) is below required threshold (${policy.constraints.min_health_score})`);
    }
  }

  // 2. Disallowed severities check
  const disallowed = policy.constraints.disallowed_severities || [];
  if (disallowed.length > 0) {
    const findingsWithSev = findings.filter((f: Finding) => {
      const isWaived = active_waivers.some((w) => w.rule_id === f.rule_id);
      return !isWaived && disallowed.includes((f.severity as string).toLowerCase());
    });

    if (findingsWithSev.length === 0) {
      passed_checks.push(`Zero un-waived findings matching disallowed severities [${disallowed.join(", ")}]`);
    } else {
      violations.push(`Found ${findingsWithSev.length} un-waived finding(s) with disallowed severities [${disallowed.join(", ")}]`);
    }
  }

  // 3. Secret zero-tolerance check
  if (policy.constraints.enforce_secret_zero_tolerance) {
    const secretFindings = findings.filter((f: Finding) => {
      const isWaived = active_waivers.some((w) => w.rule_id === f.rule_id);
      return !isWaived && (f.category.toLowerCase() === "security" || f.rule_id.includes("secret"));
    });

    if (secretFindings.length === 0) {
      passed_checks.push("Zero-tolerance secret enforcement satisfied: 0 un-waived credentials detected");
    } else {
      violations.push(`Zero-tolerance violation: ${secretFindings.length} hardcoded credential(s) detected`);
    }
  }

  // 4. Circular dependencies check
  const circularDepsCount = bundle.architecture?.circular_dependencies?.length ?? 0;
  if (policy.constraints.max_circular_dependencies !== undefined) {
    if (circularDepsCount <= policy.constraints.max_circular_dependencies) {
      passed_checks.push(`Circular dependency count (${circularDepsCount}) within limit (${policy.constraints.max_circular_dependencies})`);
    } else {
      violations.push(`Circular dependencies count (${circularDepsCount}) exceeds limit (${policy.constraints.max_circular_dependencies})`);
    }
  }

  const verdict: "passed" | "warning" | "violation" =
    violations.length > 0 ? "violation" : warnings.length > 0 ? "warning" : "passed";

  return {
    verdict,
    passed_checks,
    warnings,
    violations,
    active_waivers,
    expired_waivers,
  };
}

/**
 * Fetches all organizations from `forge-server`.
 */
export async function fetchOrganizations(
  serverUrl: string,
  token: string
): Promise<{ ok: boolean; orgs: OrgListItem[]; error?: string }> {
  try {
    const res = await fetch(`${serverUrl}/api/v1/orgs`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    if (!res.ok) {
      return { ok: false, orgs: [], error: `HTTP ${res.status}: ${res.statusText}` };
    }
    const orgs = (await res.json()) as OrgListItem[];
    return { ok: true, orgs };
  } catch (err) {
    return { ok: false, orgs: [], error: err instanceof Error ? err.message : String(err) };
  }
}

/**
 * Fetches organization summary with pre-aggregated rollup from `forge-server`.
 */
export async function fetchOrganizationSummary(
  serverUrl: string,
  idOrSlug: string,
  token: string
): Promise<{ ok: boolean; summary?: OrganizationSummary; error?: string }> {
  try {
    const res = await fetch(`${serverUrl}/api/v1/orgs/${idOrSlug}/summary`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    if (!res.ok) {
      return { ok: false, error: `HTTP ${res.status}: ${res.statusText}` };
    }
    const summary = (await res.json()) as OrganizationSummary;
    return { ok: true, summary };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

/**
 * Fetches audit event records from `forge-server`.
 */
export async function fetchAuditEvents(
  serverUrl: string,
  token: string,
  filter?: AuditEventFilter
): Promise<{ ok: boolean; events: AuditEvent[]; error?: string }> {
  try {
    const query = new URLSearchParams();
    if (filter?.org_id) query.set("org_id", filter.org_id);
    if (filter?.project_id) query.set("project_id", filter.project_id);
    if (filter?.actor) query.set("actor", filter.actor);
    if (filter?.action) query.set("action", filter.action);
    if (filter?.limit) query.set("limit", filter.limit.toString());

    const qs = query.toString();
    const url = `${serverUrl}/api/v1/audit-events${qs ? `?${qs}` : ""}`;

    const res = await fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    if (!res.ok) {
      return { ok: false, events: [], error: `HTTP ${res.status}: ${res.statusText}` };
    }
    const events = (await res.json()) as AuditEvent[];
    return { ok: true, events };
  } catch (err) {
    return { ok: false, events: [], error: err instanceof Error ? err.message : String(err) };
  }
}
