import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  parsePolicyYaml,
  isWaiverExpired,
  evaluatePolicyAgainstBundle,
  fetchOrganizations,
  fetchOrganizationSummary,
  fetchAuditEvents,
  STARTER_POLICY_YAML,
  type PolicyDocument,
} from "../src/lib/data/org.js";
import type { ReportBundle, Finding } from "@elcode-forge/schema";

describe("Governance & Policy Data Layer", () => {
  describe("parsePolicyYaml", () => {
    it("parses the starter policy YAML correctly", () => {
      const res = parsePolicyYaml(STARTER_POLICY_YAML);
      expect(res.ok).toBe(true);
      if (!res.ok) return;

      expect(res.policy.name).toBe("enterprise-standard-gate");
      expect(res.policy.version).toBe("1.0.0");
      expect(res.policy.constraints.min_health_score).toBe(80);
      expect(res.policy.constraints.enforce_secret_zero_tolerance).toBe(true);
      expect(res.policy.constraints.disallowed_severities).toContain("critical");
      expect(res.policy.constraints.disallowed_severities).toContain("high");
      expect(res.policy.constraints.forbidden_licenses).toContain("GPL-3.0-only");
      expect(res.policy.constraints.forbidden_licenses).toContain("AGPL-3.0-only");
      expect(res.policy.constraints.max_circular_dependencies).toBe(0);
      expect(res.policy.constraints.max_blast_radius_modules).toBe(5);

      expect(res.policy.waivers.length).toBe(1);
      expect(res.policy.waivers[0].rule_id).toBe("security/secret-generic-token");
      expect(res.policy.waivers[0].path_pattern).toBe("**/fixtures/**");
      expect(res.policy.waivers[0].expires_at).toBe("2027-01-01T00:00:00Z");

      expect(res.policy.overrides.length).toBe(1);
      expect(res.policy.overrides[0].rule_id).toBe("arch/no-circular-dependencies");
      expect(res.policy.overrides[0].severity).toBe("high");
      expect(res.policy.overrides[0].enabled).toBe(true);
    });

    it("handles minimal YAML with sensible defaults", () => {
      const yaml = `name: minimal-policy\nversion: 2.1.0\n`;
      const res = parsePolicyYaml(yaml);
      expect(res.ok).toBe(true);
      if (!res.ok) return;

      expect(res.policy.name).toBe("minimal-policy");
      expect(res.policy.version).toBe("2.1.0");
      expect(res.policy.constraints.min_health_score).toBeUndefined();
      expect(res.policy.waivers).toEqual([]);
      expect(res.policy.overrides).toEqual([]);
    });
  });

  describe("isWaiverExpired", () => {
    it("returns false if expires_at is undefined or empty", () => {
      expect(isWaiverExpired(undefined)).toBe(false);
      expect(isWaiverExpired("")).toBe(false);
    });

    it("returns true for past dates", () => {
      expect(isWaiverExpired("2020-01-01T00:00:00Z")).toBe(true);
    });

    it("returns false for future dates", () => {
      expect(isWaiverExpired("2099-12-31T23:59:59Z")).toBe(false);
    });

    it("returns false for invalid date strings", () => {
      expect(isWaiverExpired("invalid-date-string")).toBe(false);
    });
  });

  describe("evaluatePolicyAgainstBundle", () => {
    const mockFinding = (overrides?: Partial<Finding>): Finding => ({
      id: "f-1",
      rule_id: "sec-001",
      severity: "low",
      category: "security",
      confidence: "high",
      title: "Sample finding",
      description: "Description",
      evidence: "evidence snippet",
      recommendation: "fix recommendation",
      ...overrides,
    });

    const mockBundle = (
      score = 85,
      findings: Finding[] = [],
      circularDeps: string[][] = []
    ): ReportBundle => ({
      schema_version: 1,
      tool_version: "1.0.0",
      repository: {
        name: "test-repo",
        root_path: "/repo",
        detected_languages: ["Rust"],
        detected_frameworks: [],
        package_managers: ["cargo"],
        files: [],
        dependencies: [],
        modules: [],
        findings,
        health: {
          overall: score,
          dimensions: [],
        },
      },
      architecture: {
        circular_dependencies: circularDeps,
        high_centrality_modules: [],
      },
      rules: [],
      hotspots: [],
    });

    const samplePolicy: PolicyDocument = {
      name: "strict-gate",
      version: "1.0.0",
      constraints: {
        min_health_score: 80,
        enforce_secret_zero_tolerance: true,
        disallowed_severities: ["critical", "high"],
        max_circular_dependencies: 0,
      },
      waivers: [],
      overrides: [],
    };

    it("returns passed when bundle is null", () => {
      const res = evaluatePolicyAgainstBundle(samplePolicy, null);
      expect(res.verdict).toBe("passed");
      expect(res.passed_checks.length).toBeGreaterThan(0);
      expect(res.violations.length).toBe(0);
    });

    it("passes when bundle meets all policy requirements", () => {
      const bundle = mockBundle(90, [mockFinding({ severity: "low", category: "style" })]);
      const res = evaluatePolicyAgainstBundle(samplePolicy, bundle);
      expect(res.verdict).toBe("passed");
      expect(res.violations.length).toBe(0);
      expect(res.passed_checks.length).toBe(4);
    });

    it("detects health score below threshold", () => {
      const bundle = mockBundle(75);
      const res = evaluatePolicyAgainstBundle(samplePolicy, bundle);
      expect(res.verdict).toBe("violation");
      expect(res.violations).toContain(
        "Repository health score (75) is below required threshold (80)"
      );
    });

    it("detects disallowed severity violations", () => {
      const bundle = mockBundle(85, [
        mockFinding({ rule_id: "sec-vuln", severity: "critical", category: "quality" }),
      ]);
      const res = evaluatePolicyAgainstBundle(samplePolicy, bundle);
      expect(res.verdict).toBe("violation");
      expect(res.violations.some((v: string) => v.includes("disallowed severities"))).toBe(true);
    });

    it("respects active waivers for disallowed severities", () => {
      const policyWithWaiver: PolicyDocument = {
        ...samplePolicy,
        waivers: [
          {
            rule_id: "sec-vuln",
            expires_at: "2099-01-01T00:00:00Z",
            justification: "Known legacy issue with compensating controls",
          },
        ],
      };
      const bundle = mockBundle(85, [
        mockFinding({ rule_id: "sec-vuln", severity: "critical", category: "quality" }),
      ]);
      const res = evaluatePolicyAgainstBundle(policyWithWaiver, bundle);
      expect(res.verdict).toBe("passed");
      expect(res.violations.length).toBe(0);
      expect(res.active_waivers.length).toBe(1);
    });

    it("warns about expired waivers and does not waive findings", () => {
      const policyWithExpiredWaiver: PolicyDocument = {
        ...samplePolicy,
        waivers: [
          {
            rule_id: "sec-vuln",
            expires_at: "2020-01-01T00:00:00Z",
            justification: "Expired waiver",
          },
        ],
      };
      const bundle = mockBundle(85, [
        mockFinding({ rule_id: "sec-vuln", severity: "critical", category: "quality" }),
      ]);
      const res = evaluatePolicyAgainstBundle(policyWithExpiredWaiver, bundle);
      expect(res.verdict).toBe("violation");
      expect(res.expired_waivers.length).toBe(1);
      expect(res.warnings.length).toBe(1);
      expect(res.warnings[0]).toContain("expired on 2020-01-01T00:00:00Z");
    });

    it("enforces secret zero-tolerance violation", () => {
      const bundle = mockBundle(95, [
        mockFinding({
          rule_id: "rules/secret-token",
          severity: "low",
          category: "security",
        }),
      ]);
      const res = evaluatePolicyAgainstBundle(samplePolicy, bundle);
      expect(res.verdict).toBe("violation");
      expect(res.violations.some((v: string) => v.includes("Zero-tolerance violation"))).toBe(true);
    });

    it("enforces circular dependency threshold", () => {
      const bundle = mockBundle(95, [], [["crates/a", "crates/b", "crates/a"]]);
      const res = evaluatePolicyAgainstBundle(samplePolicy, bundle);
      expect(res.verdict).toBe("violation");
      expect(res.violations.some((v: string) => v.includes("Circular dependencies count"))).toBe(true);
    });
  });

  describe("Server API Integration Helpers", () => {
    beforeEach(() => {
      vi.restoreAllMocks();
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("fetchOrganizations handles success response", async () => {
      const mockOrgs = [
        {
          id: "org-1",
          name: "Acme Corp",
          slug: "acme",
          health_score: 88,
          total_projects: 3,
          total_vulnerabilities: 2,
          created_at: 1000,
          updated_at: 2000,
        },
      ];

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockOrgs,
      });

      const res = await fetchOrganizations("http://localhost:8000", "test-token");
      expect(res.ok).toBe(true);
      expect(res.orgs).toEqual(mockOrgs);
      expect(global.fetch).toHaveBeenCalledWith("http://localhost:8000/api/v1/orgs", {
        headers: { Authorization: "Bearer test-token" },
      });
    });

    it("fetchOrganizations handles HTTP error", async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: "Forbidden",
      });

      const res = await fetchOrganizations("http://localhost:8000", "bad-token");
      expect(res.ok).toBe(false);
      expect(res.error).toContain("HTTP 403: Forbidden");
    });

    it("fetchOrganizationSummary handles success and failure", async () => {
      const mockSummary = {
        id: "org-1",
        name: "Acme Corp",
        slug: "acme",
        health_score: 88,
        total_projects: 3,
        total_vulnerabilities: 2,
        cross_repo_vulnerabilities: [
          {
            rule_id: "sec-001",
            severity: "critical",
            title: "CVE-2026-0001",
            category: "security",
            affected_projects: ["proj-a", "proj-b"],
            occurrence_count: 2,
          },
        ],
        project_ids: ["p1", "p2", "p3"],
        updated_at: 2000,
      };

      global.fetch = vi.fn().mockResolvedValueOnce({
        ok: true,
        json: async () => mockSummary,
      });

      const res = await fetchOrganizationSummary("http://localhost:8000", "acme", "token");
      expect(res.ok).toBe(true);
      expect(res.summary?.cross_repo_vulnerabilities.length).toBe(1);

      // Failure case
      global.fetch = vi.fn().mockRejectedValueOnce(new Error("Network failed"));
      const failRes = await fetchOrganizationSummary("http://localhost:8000", "acme", "token");
      expect(failRes.ok).toBe(false);
      expect(failRes.error).toBe("Network failed");
    });

    it("fetchAuditEvents handles query parameters properly", async () => {
      const mockEvents = [
        {
          id: "evt-1",
          org_id: "org-1",
          project_id: "proj-1",
          actor: "alice@example.com",
          action: "policy.update",
          resource_type: "policy",
          resource_id: "pol-1",
          details_json: "{}",
          created_at: 3000,
        },
      ];

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockEvents,
      });

      const res = await fetchAuditEvents("http://localhost:8000", "token", {
        org_id: "org-1",
        actor: "alice@example.com",
        limit: 25,
      });

      expect(res.ok).toBe(true);
      expect(res.events).toEqual(mockEvents);
      expect(global.fetch).toHaveBeenCalledWith(
        "http://localhost:8000/api/v1/audit-events?org_id=org-1&actor=alice%40example.com&limit=25",
        {
          headers: { Authorization: "Bearer token" },
        }
      );
    });
  });
});
