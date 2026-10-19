import type { Finding, Severity, Confidence } from "@elcode-forge/schema";

export interface FindingFilter {
  severity: Severity[];
  confidence: Confidence[];
  rulePrefix?: string;
  pathPrefix?: string;
  category?: string;
}

const VALID_SEVERITIES: Set<Severity> = new Set(["critical", "high", "medium", "low", "info"]);
const VALID_CONFIDENCES: Set<Confidence> = new Set(["certain", "high", "medium", "low"]);

/**
 * Parse URLSearchParams into a structured FindingFilter.
 */
export function parseFilter(params: URLSearchParams): FindingFilter {
  const sevParam = params.get("severity");
  const confParam = params.get("confidence");
  const rulePrefix = params.get("rulePrefix") || params.get("rule_prefix") || undefined;
  const pathPrefix = params.get("pathPrefix") || params.get("path_prefix") || undefined;
  const category = params.get("category") || undefined;

  const severity: Severity[] = [];
  if (sevParam) {
    for (const s of sevParam.split(",")) {
      const lower = s.trim().toLowerCase() as Severity;
      if (VALID_SEVERITIES.has(lower)) {
        severity.push(lower);
      }
    }
  }

  const confidence: Confidence[] = [];
  if (confParam) {
    for (const c of confParam.split(",")) {
      const lower = c.trim().toLowerCase() as Confidence;
      if (VALID_CONFIDENCES.has(lower)) {
        confidence.push(lower);
      }
    }
  }

  return {
    severity,
    confidence,
    rulePrefix: rulePrefix?.trim() || undefined,
    pathPrefix: pathPrefix?.trim() || undefined,
    category: category?.trim() || undefined,
  };
}

/**
 * Serialize a FindingFilter back into URLSearchParams.
 */
export function serializeFilter(filter: FindingFilter): URLSearchParams {
  const params = new URLSearchParams();

  if (filter.severity && filter.severity.length > 0) {
    params.set("severity", filter.severity.join(","));
  }

  if (filter.confidence && filter.confidence.length > 0) {
    params.set("confidence", filter.confidence.join(","));
  }

  if (filter.rulePrefix) {
    params.set("rulePrefix", filter.rulePrefix);
  }

  if (filter.pathPrefix) {
    params.set("pathPrefix", filter.pathPrefix);
  }

  if (filter.category) {
    params.set("category", filter.category);
  }

  return params;
}

/**
 * Apply the FindingFilter to a list of findings, preserving original stable order.
 */
export function applyFilter(findings: Finding[], filter: FindingFilter): Finding[] {
  if (!findings || findings.length === 0) return [];

  const sevSet = filter.severity && filter.severity.length > 0 ? new Set(filter.severity) : null;
  const confSet = filter.confidence && filter.confidence.length > 0 ? new Set(filter.confidence) : null;
  const rulePrefix = filter.rulePrefix ? filter.rulePrefix.toLowerCase() : null;
  const pathPrefix = filter.pathPrefix ? filter.pathPrefix.toLowerCase() : null;
  const category = filter.category ? filter.category.toLowerCase() : null;

  return findings.filter((f) => {
    if (sevSet && !sevSet.has(f.severity)) {
      return false;
    }

    if (confSet && !confSet.has(f.confidence)) {
      return false;
    }

    if (rulePrefix && !f.rule_id.toLowerCase().startsWith(rulePrefix)) {
      return false;
    }

    if (pathPrefix) {
      const p = f.location?.file_path?.toLowerCase();
      if (!p || !p.includes(pathPrefix)) {
        return false;
      }
    }

    if (category && f.category?.toLowerCase() !== category) {
      return false;
    }

    return true;
  });
}
