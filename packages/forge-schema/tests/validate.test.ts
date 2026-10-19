import { describe, expect, it } from "vitest";
import { parseReportBundle, SUPPORTED_SCHEMA_VERSION } from "../src/index";

describe("parseReportBundle", () => {
  const validBundle = {
    schema_version: 1,
    tool_version: "0.2.0",
    revision: {
      head_sha: "abcd1234abcd1234abcd1234abcd1234abcd1234",
      commit_unix_ts: 1729000000,
    },
    repository: {
      root_path: "/repo",
      name: "sample-repo",
      detected_languages: ["typescript"],
      detected_frameworks: ["nextjs"],
      package_managers: ["pnpm"],
      files: [
        {
          relative_path: "src/index.ts",
          language: "typescript",
          size_bytes: 120,
          line_count: 10,
          content_hash: "blake3hash",
          is_test: false,
        },
      ],
      modules: [
        {
          id: "src",
          name: "src",
          root_path: "src",
          files: ["src/index.ts"],
          imports: [],
          exports: [],
        },
      ],
      dependencies: [
        {
          name: "react",
          version_req: "^18.0.0",
          resolved_version: "18.2.0",
          ecosystem: "npm",
          is_dev: false,
          is_direct: true,
        },
      ],
      findings: [
        {
          id: "F-001",
          category: "architecture",
          severity: "high",
          confidence: "certain",
          title: "Violation",
          description: "Desc",
          evidence: "Evidence text",
          location: {
            file_path: "src/index.ts",
            start_line: 1,
            end_line: 1,
          },
          recommendation: "Fix it",
          rule_id: "arch/sample",
        },
      ],
      health: {
        overall: 85,
        dimensions: [
          {
            dimension: "Architecture",
            score: 80,
            weight_pct: 20,
            reasons: ["Deduction 20"],
          },
        ],
      },
    },
    architecture: {
      circular_dependencies: [["src/a", "src/b"]],
      high_centrality_modules: ["src/db"],
    },
    hotspots: [
      {
        path: "src/index.ts",
        commit_count: 5,
        contributors_count: 2,
      },
    ],
    change_impact: {
      changed_files: [
        {
          path: "src/index.ts",
          added_lines: 5,
          deleted_lines: 2,
          is_new: false,
          is_deleted: false,
        },
      ],
      directly_affected_modules: ["src"],
      transitively_affected_modules: [],
      changed_files_without_tests: [],
    },
    change_risk: {
      level: "LOW",
      score: 15,
      reasons: ["Minimal changes"],
    },
  };

  it("validates a compliant ReportBundle successfully", () => {
    const result = parseReportBundle(validBundle);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.bundle.schema_version).toBe(SUPPORTED_SCHEMA_VERSION);
      expect(result.bundle.repository.name).toBe("sample-repo");
    }
  });

  it("rejects non-object or null input", () => {
    expect(parseReportBundle(null).ok).toBe(false);
    expect(parseReportBundle("not-json").ok).toBe(false);
    expect(parseReportBundle(42).ok).toBe(false);
  });

  it("rejects unsupported schema version", () => {
    const invalidVersion = { ...validBundle, schema_version: 99 };
    const result = parseReportBundle(invalidVersion);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.errors[0]).toContain("Unsupported schema_version");
    }
  });

  it("rejects missing required repository field", () => {
    const missingRepo = { ...validBundle, repository: undefined };
    const result = parseReportBundle(missingRepo);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.errors.length).toBeGreaterThan(0);
    }
  });

  it("rejects invalid severity enum value", () => {
    const invalidSeverity = JSON.parse(JSON.stringify(validBundle));
    invalidSeverity.repository.findings[0].severity = "ULTRA_CRITICAL";
    const result = parseReportBundle(invalidSeverity);
    expect(result.ok).toBe(false);
  });
});
