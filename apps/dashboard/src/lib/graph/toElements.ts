import type { ReportBundle } from "@elcode-forge/schema";

export interface GraphNodeData {
  id: string;
  name: string;
  rootPath: string;
  fileCount: number;
  exportCount: number;
  importCount: number;
  files: string[];
  exports: string[];
  imports: string[];
  hasCycle: boolean;
}

export interface GraphNode {
  data: GraphNodeData;
}

export interface GraphEdgeData {
  id: string;
  source: string;
  target: string;
  inCycle: boolean;
}

export interface GraphEdge {
  data: GraphEdgeData;
}

export interface GraphElements {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

/**
 * Pure transformation from ReportBundle to Cytoscape GraphElements.
 * Flags edges and nodes participating in circular dependencies with `inCycle: true` / `hasCycle: true`.
 */
export function toElements(bundle: ReportBundle): GraphElements {
  if (!bundle?.repository?.modules) {
    return { nodes: [], edges: [] };
  }

  // Pre-process circular dependency paths to find all cyclic edges (u -> v)
  // bundle.architecture.circular_dependencies is string[][] e.g. [["crates/forge-core", "crates/forge-analyzer", "crates/forge-core"]]
  const cycleEdges = new Set<string>();
  const cycleNodes = new Set<string>();

  const cycles = bundle.architecture?.circular_dependencies ?? [];
  for (const cycle of cycles) {
    if (!Array.isArray(cycle) || cycle.length === 0) continue;
    for (let i = 0; i < cycle.length - 1; i++) {
      const u = cycle[i];
      const v = cycle[i + 1];
      cycleEdges.add(`${u}->${v}`);
      cycleNodes.add(u);
      cycleNodes.add(v);
    }
  }

  const moduleMap = new Map<string, typeof bundle.repository.modules[0]>();
  for (const mod of bundle.repository.modules) {
    // Both mod.id and mod.name could be used as identifier; default to mod.id
    moduleMap.set(mod.id, mod);
  }

  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  const seenEdges = new Set<string>();

  for (const mod of bundle.repository.modules) {
    const isNodeInCycle = cycleNodes.has(mod.id) || cycleNodes.has(mod.name);
    nodes.push({
      data: {
        id: mod.id,
        name: mod.name || mod.id,
        rootPath: mod.root_path,
        fileCount: mod.files ? mod.files.length : 0,
        exportCount: mod.exports ? mod.exports.length : 0,
        importCount: mod.imports ? mod.imports.length : 0,
        files: mod.files || [],
        exports: mod.exports || [],
        imports: mod.imports || [],
        hasCycle: isNodeInCycle,
      },
    });

    if (Array.isArray(mod.imports)) {
      for (const targetId of mod.imports) {
        // Find if target is an internal module
        const targetMod = moduleMap.get(targetId) ||
          Array.from(moduleMap.values()).find((m) => m.name === targetId);

        const resolvedTargetId = targetMod ? targetMod.id : targetId;
        const edgeKey = `${mod.id}->${resolvedTargetId}`;

        if (!seenEdges.has(edgeKey)) {
          seenEdges.add(edgeKey);
          const inCycle = cycleEdges.has(edgeKey) ||
            (targetMod && (cycleEdges.has(`${mod.name}->${targetMod.name}`) ||
              cycleEdges.has(`${mod.id}->${targetMod.name}`) ||
              cycleEdges.has(`${mod.name}->${resolvedTargetId}`)));

          edges.push({
            data: {
              id: `edge-${edgeKey}`,
              source: mod.id,
              target: resolvedTargetId,
              inCycle: Boolean(inCycle),
            },
          });
        }
      }
    }
  }

  return { nodes, edges };
}
