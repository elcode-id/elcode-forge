import { describe, expect, it } from "vitest";
import { categorizeLicense, getLicenseBadgeClass } from "../src/lib/data/license.js";
import { generateClientCycloneDx, generateClientSpdx, generatePurl } from "../src/lib/data/sbom.js";
import type { Dependency, ReportBundle } from "@elcode-forge/schema";

describe("License categorization & badge styling", () => {
  it("categorizes permissive licenses correctly", () => {
    expect(categorizeLicense("MIT")).toBe("Permissive");
    expect(categorizeLicense("Apache-2.0")).toBe("Permissive");
    expect(categorizeLicense("BSD-3-Clause")).toBe("Permissive");
    expect(categorizeLicense("ISC")).toBe("Permissive");
    expect(categorizeLicense("Unlicense")).toBe("Permissive");
    expect(categorizeLicense("CC0-1.0")).toBe("Permissive");
  });

  it("categorizes copyleft licenses accurately", () => {
    expect(categorizeLicense("LGPL-2.1")).toBe("WeakCopyleft");
    expect(categorizeLicense("LGPL-3.0")).toBe("WeakCopyleft");
    expect(categorizeLicense("MPL-2.0")).toBe("WeakCopyleft");
    expect(categorizeLicense("GPL-2.0")).toBe("StrongCopyleft");
    expect(categorizeLicense("GPL-3.0")).toBe("StrongCopyleft");
    expect(categorizeLicense("AGPL-3.0")).toBe("StrongCopyleft");
  });

  it("handles compound SPDX expressions", () => {
    expect(categorizeLicense("MIT OR Apache-2.0")).toBe("Permissive");
    expect(categorizeLicense("MIT OR GPL-3.0")).toBe("Permissive");
    expect(categorizeLicense("MIT AND GPL-3.0")).toBe("StrongCopyleft");
  });

  it("assigns distinct tailwind classes per category", () => {
    expect(getLicenseBadgeClass("Permissive")).toContain("emerald");
    expect(getLicenseBadgeClass("WeakCopyleft")).toContain("amber");
    expect(getLicenseBadgeClass("StrongCopyleft")).toContain("rose");
    expect(getLicenseBadgeClass("Proprietary")).toContain("purple");
  });
});

describe("Client-side SBOM generators", () => {
  const mockDeps: Dependency[] = [
    {
      name: "lodash",
      ecosystem: "npm",
      version_req: "^4.17.21",
      resolved_version: "4.17.21",
      is_direct: true,
      is_dev: false,
    },
    {
      name: "@scope/pkg",
      ecosystem: "npm",
      version_req: "1.0.0",
      resolved_version: "1.0.0",
      is_direct: true,
      is_dev: false,
    },
    {
      name: "requests",
      ecosystem: "pypi",
      version_req: "==2.28.0",
      resolved_version: "2.28.0",
      is_direct: true,
      is_dev: true,
    },
    {
      name: "github.com/gin-gonic/gin",
      ecosystem: "go",
      version_req: "v1.8.0",
      resolved_version: "v1.8.0",
      is_direct: true,
      is_dev: false,
    },
  ];

  const mockBundle: ReportBundle = {
    schema_version: 1,
    tool_version: "1.0.0",
    repository: {
      name: "test-dashboard-app",
      root_path: "/workspace/test-app",
      detected_languages: ["typescript"],
      detected_frameworks: ["sveltekit"],
      package_managers: ["pnpm"],
      files: [],
      modules: [],
      dependencies: mockDeps,
      findings: [],
    },
    architecture: {
      circular_dependencies: [],
      high_centrality_modules: [],
    },
    hotspots: [],
    rules: [],
  };

  it("generates canonical PURLs across ecosystems", () => {
    expect(generatePurl(mockDeps[0])).toBe("pkg:npm/lodash@4.17.21");
    expect(generatePurl(mockDeps[1])).toBe("pkg:npm/%40scope/pkg@1.0.0");
    expect(generatePurl(mockDeps[2])).toBe("pkg:pypi/requests@2.28.0");
    expect(generatePurl(mockDeps[3])).toBe("pkg:golang/github.com/gin-gonic/gin@v1.8.0");
  });

  it("produces valid CycloneDX v1.5 JSON", () => {
    const jsonStr = generateClientCycloneDx(mockBundle);
    const parsed = JSON.parse(jsonStr);

    expect(parsed.bomFormat).toBe("CycloneDX");
    expect(parsed.specVersion).toBe("1.5");
    expect(parsed.metadata.component.name).toBe("test-dashboard-app");
    expect(parsed.components).toHaveLength(4);
    expect(parsed.components[0].purl).toBe("pkg:npm/lodash@4.17.21");
    expect(parsed.components[2].scope).toBe("optional"); // dev dep
  });

  it("produces valid SPDX v2.3 JSON", () => {
    const jsonStr = generateClientSpdx(mockBundle);
    const parsed = JSON.parse(jsonStr);

    expect(parsed.spdxVersion).toBe("SPDX-2.3");
    expect(parsed.name).toBe("test-dashboard-app");
    expect(parsed.packages).toHaveLength(5); // root + 4 deps
    expect(parsed.relationships).toHaveLength(5);
  });
});
