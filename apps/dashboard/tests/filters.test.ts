import { describe, it, expect } from "vitest";
import { parseFilter, serializeFilter, applyFilter, detectFileLanguage } from "../src/lib/filters.js";
import type { Finding } from "@elcode-forge/schema";

describe("Finding filters helper", () => {
  it("parses and serializes URLSearchParams accurately", () => {
    const params = new URLSearchParams("severity=critical,high&confidence=certain&rulePrefix=arch/&pathPrefix=src/db");
    const filter = parseFilter(params);

    expect(filter.severity).toEqual(["critical", "high"]);
    expect(filter.confidence).toEqual(["certain"]);
    expect(filter.rulePrefix).toBe("arch/");
    expect(filter.pathPrefix).toBe("src/db");

    const serialized = serializeFilter(filter);
    expect(serialized.get("severity")).toBe("critical,high");
    expect(serialized.get("confidence")).toBe("certain");
    expect(serialized.get("rulePrefix")).toBe("arch/");
    expect(serialized.get("pathPrefix")).toBe("src/db");
  });

  it("applies filters accurately with stable ordering preserved", () => {
    const findings: Finding[] = [
      {
        id: "f-1",
        rule_id: "arch/no-direct-db-in-ui",
        severity: "critical",
        confidence: "certain",
        title: "Layer violation",
        description: "UI component directly imports database client",
        evidence: "import db from '../db/client'",
        recommendation: "Move database access to a service layer",
        category: "architecture",
        location: {
          file_path: "src/components/UserCard.tsx",
          start_line: 5,
        },
      },
      {
        id: "f-2",
        rule_id: "security/hardcoded-secret",
        severity: "critical",
        confidence: "high",
        title: "Hardcoded API Token",
        description: "Bearer token detected in source code",
        evidence: "apiKey = ****REDACTED****",
        recommendation: "Use environment variables",
        category: "security",
        location: {
          file_path: "src/services/api.ts",
          start_line: 12,
        },
      },
      {
        id: "f-3",
        rule_id: "deps/unused-dependency",
        severity: "low",
        confidence: "certain",
        title: "Unused Dependency",
        description: "Dependency unused-analytics-sdk is never imported",
        evidence: "unused-analytics-sdk",
        recommendation: "Remove unused dependency",
        category: "dependencies",
        location: {
          file_path: "package.json",
        },
      },
    ];

    // Filter by severity
    const criticals = applyFilter(findings, { severity: ["critical"], confidence: [] });
    expect(criticals.map((f: Finding) => f.id)).toEqual(["f-1", "f-2"]);

    // Filter by rule prefix
    const archs = applyFilter(findings, { severity: [], confidence: [], rulePrefix: "arch/" });
    expect(archs.map((f: Finding) => f.id)).toEqual(["f-1"]);

    // Filter by path prefix
    const serviceFindings = applyFilter(findings, { severity: [], confidence: [], pathPrefix: "services" });
    expect(serviceFindings.map((f: Finding) => f.id)).toEqual(["f-2"]);

    // Combined filter
    const combined = applyFilter(findings, {
      severity: ["critical"],
      confidence: ["certain"],
      rulePrefix: "arch/",
    });
    expect(combined.map((f: Finding) => f.id)).toEqual(["f-1"]);
  });

  it("detects programming language accurately from file paths and manifests", () => {
    expect(detectFileLanguage("src/components/Card.tsx")).toBe("typescript");
    expect(detectFileLanguage("app/routers/items.py")).toBe("python");
    expect(detectFileLanguage("cmd/server/main.go")).toBe("go");
    expect(detectFileLanguage("crates/engine/src/lib.rs")).toBe("rust");
    expect(detectFileLanguage("app/Http/Controllers/UserController.php")).toBe("php");
    expect(detectFileLanguage("src/index.js")).toBe("javascript");
    expect(detectFileLanguage("src/routes/+page.svelte")).toBe("svelte");
    expect(detectFileLanguage("src/App.vue")).toBe("vue");
    expect(detectFileLanguage("Cargo.toml")).toBe("rust");
    expect(detectFileLanguage("go.mod")).toBe("go");
    expect(detectFileLanguage("requirements.txt")).toBe("python");
    expect(detectFileLanguage("composer.json")).toBe("php");
    expect(detectFileLanguage("package.json")).toBe("javascript");
    expect(detectFileLanguage("README.md")).toBe(null);
  });

  it("parses, serializes, and filters findings by language accurately", () => {
    const params = new URLSearchParams("language=python");
    const filter = parseFilter(params);
    expect(filter.language).toBe("python");

    const serialized = serializeFilter(filter);
    expect(serialized.get("language")).toBe("python");

    const findings: Finding[] = [
      {
        id: "py-1",
        rule_id: "security/python-unsafe-eval",
        severity: "critical",
        confidence: "certain",
        title: "Unsafe execution",
        description: "Unsafe dynamic execution detected in Python",
        evidence: "execute_dynamic(input)",
        recommendation: "Avoid eval",
        category: "security",
        location: { file_path: "app/main.py", start_line: 10 },
      },
      {
        id: "go-1",
        rule_id: "arch/no-circular-dependencies",
        severity: "high",
        confidence: "certain",
        title: "Go cycle",
        description: "Circular package import",
        evidence: "a <-> b",
        recommendation: "Break cycle",
        category: "architecture",
        location: { file_path: "internal/pkg/a.go", start_line: 1 },
      },
    ];

    const pyFiltered = applyFilter(findings, { severity: [], confidence: [], language: "python" });
    expect(pyFiltered.map((f: Finding) => f.id)).toEqual(["py-1"]);

    const goFiltered = applyFilter(findings, { severity: [], confidence: [], language: "go" });
    expect(goFiltered.map((f: Finding) => f.id)).toEqual(["go-1"]);

    const rsFiltered = applyFilter(findings, { severity: [], confidence: [], language: "rust" });
    expect(rsFiltered).toEqual([]);
  });
});
