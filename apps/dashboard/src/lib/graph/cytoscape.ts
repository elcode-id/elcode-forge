import cytoscape, { type Core, type EventObject } from "cytoscape";
import dagre from "cytoscape-dagre";
import type { GraphElements, GraphNode, GraphEdge } from "./toElements.js";

// Register dagre layout extension once safely
let dagreRegistered = false;
function ensureDagre() {
  if (!dagreRegistered) {
    try {
      cytoscape.use(dagre);
      dagreRegistered = true;
    } catch {
      // already registered or failed
    }
  }
}

export interface CytoscapeOptions {
  container: HTMLElement;
  elements: GraphElements;
  onSelectNode?: (nodeId: string | null) => void;
}

/**
 * Initialize Cytoscape with ELCODE Dark Technical Theme and Dagre layout.
 */
export function initCytoscape(options: CytoscapeOptions): Core {
  ensureDagre();

  const { container, elements, onSelectNode } = options;

  const cy = cytoscape({
    container,
    elements: [
      ...elements.nodes.map((n: GraphNode) => ({
        group: "nodes" as const,
        data: n.data,
      })),
      ...elements.edges.map((e: GraphEdge) => ({
        group: "edges" as const,
        data: e.data,
      })),
    ],
    boxSelectionEnabled: false,
    autounselectify: false,
    layout: {
      name: "dagre",
      // @ts-expect-error dagre layout property
      rankDir: "TB",
      nodeSep: 40,
      rankSep: 60,
      padding: 30,
    },
    style: [
      {
        selector: "node",
        style: {
          label: "data(name)",
          color: "#e2e8f0",
          "font-family": "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
          "font-size": "11px",
          "text-valign": "center",
          "text-halign": "center",
          "background-color": "#161f30",
          "border-color": "#26354f",
          "border-width": 1.5,
          width: "label",
          height: 36,
          padding: "10px",
          shape: "round-rectangle",
          "transition-property": "background-color, border-color, border-width",
          "transition-duration": 150,
        },
      },
      {
        selector: "node[?hasCycle]",
        style: {
          "border-color": "#ef4444",
          "border-width": 2,
          color: "#fca5a5",
          "background-color": "#2a151b",
        },
      },
      {
        selector: "node:selected",
        style: {
          "border-color": "#38bdf8",
          "border-width": 2.5,
          "background-color": "#1e293b",
          color: "#ffffff",
        },
      },
      {
        selector: "edge",
        style: {
          width: 1.5,
          "line-color": "#334155",
          "target-arrow-color": "#334155",
          "target-arrow-shape": "triangle",
          "curve-style": "bezier",
          "arrow-scale": 0.9,
          "transition-property": "line-color, target-arrow-color, width",
          "transition-duration": 150,
        },
      },
      {
        selector: "edge[?inCycle]",
        style: {
          width: 2.5,
          "line-color": "#ef4444",
          "target-arrow-color": "#ef4444",
          "line-style": "solid",
        },
      },
    ],
  });

  if (onSelectNode) {
    cy.on("tap", "node", (evt: EventObject) => {
      const node = evt.target;
      onSelectNode(node.id());
    });

    cy.on("tap", (evt: EventObject) => {
      if (evt.target === cy) {
        onSelectNode(null);
      }
    });
  }

  return cy;
}
