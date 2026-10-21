import { describe, it, expect } from "vitest";
import { toElements, type GraphNode, type GraphEdge } from "../src/lib/graph/toElements.js";
import type { ReportBundle } from "@elcode-forge/schema";

describe("toElements pure transformation", () => {
  it("handles null or empty bundles cleanly", () => {
    const res1 = toElements(null as unknown as ReportBundle);
    expect(res1.nodes).toEqual([]);
    expect(res1.edges).toEqual([]);

    const res2 = toElements({
      schema_version: 1,
      tool_version: "0.2.0",
      architecture: { circular_dependencies: [], high_centrality_modules: [] },
      hotspots: [],
      rules: [],
      repository: {
        name: "empty-repo",
        root_path: "/workspace",
        detected_languages: [],
        detected_frameworks: [],
        package_managers: [],
        dependencies: [],
        files: [],
        findings: [],
        modules: [],
      },
    });
    expect(res2.nodes).toEqual([]);
    expect(res2.edges).toEqual([]);
  });

  it("converts modules and directed edges, tagging cycles correctly", () => {
    const bundle: ReportBundle = {
      schema_version: 1,
      tool_version: "0.2.0",
      architecture: {
        circular_dependencies: [
          ["crates/forge-core", "crates/forge-analyzer", "crates/forge-core"],
        ],
        high_centrality_modules: ["crates/forge-core"],
      },
      hotspots: [],
      rules: [],
      repository: {
        name: "test-repo",
        root_path: "/workspace",
        detected_languages: ["Rust"],
        detected_frameworks: [],
        package_managers: ["cargo"],
        dependencies: [],
        files: [],
        findings: [],
        modules: [
          {
            id: "crates/forge-core",
            name: "forge-core",
            root_path: "crates/forge-core",
            files: ["src/lib.rs", "src/ir.rs"],
            exports: ["Repository"],
            imports: ["crates/forge-analyzer"],
          },
          {
            id: "crates/forge-analyzer",
            name: "forge-analyzer",
            root_path: "crates/forge-analyzer",
            files: ["src/lib.rs"],
            exports: ["scan"],
            imports: ["crates/forge-core"],
          },
          {
            id: "crates/forge-cli",
            name: "forge-cli",
            root_path: "crates/forge-cli",
            files: ["src/main.rs"],
            exports: [],
            imports: ["crates/forge-core"],
          },
        ],
      },
    };

    const elements = toElements(bundle);

    expect(elements.nodes.length).toBe(3);
    const coreNode = elements.nodes.find((n: GraphNode) => n.data.id === "crates/forge-core");
    expect(coreNode).toBeDefined();
    expect(coreNode?.data.hasCycle).toBe(true);
    expect(coreNode?.data.fileCount).toBe(2);

    const cliNode = elements.nodes.find((n: GraphNode) => n.data.id === "crates/forge-cli");
    expect(cliNode).toBeDefined();
    expect(cliNode?.data.hasCycle).toBe(false);

    expect(elements.edges.length).toBe(3);

    // Edge forge-core -> forge-analyzer should be marked inCycle: true
    const cycleEdge1 = elements.edges.find(
      (e: GraphEdge) => e.data.source === "crates/forge-core" && e.data.target === "crates/forge-analyzer"
    );
    expect(cycleEdge1?.data.inCycle).toBe(true);

    // Edge forge-analyzer -> forge-core should be marked inCycle: true
    const cycleEdge2 = elements.edges.find(
      (e: GraphEdge) => e.data.source === "crates/forge-analyzer" && e.data.target === "crates/forge-core"
    );
    expect(cycleEdge2?.data.inCycle).toBe(true);

    // Edge forge-cli -> forge-core is not in cycle
    const nonCycleEdge = elements.edges.find(
      (e: GraphEdge) => e.data.source === "crates/forge-cli" && e.data.target === "crates/forge-core"
    );
    expect(nonCycleEdge?.data.inCycle).toBe(false);
  });
});
