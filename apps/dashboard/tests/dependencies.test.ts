import { describe, it, expect } from "vitest";
import type { Dependency } from "@elcode-forge/schema";

describe("Multi-ecosystem dependencies logic", () => {
  const dependencies: Dependency[] = [
    {
      name: "react",
      ecosystem: "npm",
      version_req: "^18.2.0",
      resolved_version: "18.2.0",
      is_direct: true,
      is_dev: false,
    },
    {
      name: "vitest",
      ecosystem: "npm",
      version_req: "^1.0.0",
      resolved_version: "1.0.0",
      is_direct: true,
      is_dev: true,
    },
    {
      name: "fastapi",
      ecosystem: "pypi",
      version_req: "==0.100.0",
      resolved_version: "0.100.0",
      is_direct: true,
      is_dev: false,
    },
    {
      name: "github.com/gin-gonic/gin",
      ecosystem: "go",
      version_req: "v1.9.1",
      resolved_version: "v1.9.1",
      is_direct: true,
      is_dev: false,
    },
    {
      name: "serde",
      ecosystem: "cargo",
      version_req: "1.0",
      resolved_version: "1.0.197",
      is_direct: true,
      is_dev: false,
    },
  ];

  it("extracts distinct sorted ecosystems from dependencies", () => {
    const ecosystems = Array.from(new Set(dependencies.map((d) => d.ecosystem.toLowerCase()))).sort();
    expect(ecosystems).toEqual(["cargo", "go", "npm", "pypi"]);
  });

  it("filters dependencies accurately by ecosystem", () => {
    const npmDeps = dependencies.filter((d) => d.ecosystem.toLowerCase() === "npm");
    expect(npmDeps.map((d) => d.name)).toEqual(["react", "vitest"]);

    const pyDeps = dependencies.filter((d) => d.ecosystem.toLowerCase() === "pypi");
    expect(pyDeps.map((d) => d.name)).toEqual(["fastapi"]);

    const goDeps = dependencies.filter((d) => d.ecosystem.toLowerCase() === "go");
    expect(goDeps.map((d) => d.name)).toEqual(["github.com/gin-gonic/gin"]);

    const cargoDeps = dependencies.filter((d) => d.ecosystem.toLowerCase() === "cargo");
    expect(cargoDeps.map((d) => d.name)).toEqual(["serde"]);
  });

  it("filters dependencies by direct vs dev type", () => {
    const directProd = dependencies.filter((d) => d.is_direct && !d.is_dev);
    expect(directProd.map((d) => d.name)).toEqual([
      "react",
      "fastapi",
      "github.com/gin-gonic/gin",
      "serde",
    ]);

    const devDeps = dependencies.filter((d) => d.is_dev);
    expect(devDeps.map((d) => d.name)).toEqual(["vitest"]);
  });
});
