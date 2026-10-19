import { describe, expect, it } from "vitest";
import * as components from "../src/index.js";

describe("forge-ui exports", () => {
  it("exports all 6 core UI component primitives", () => {
    expect(components.ScoreRing).toBeDefined();
    expect(components.SeverityBadge).toBeDefined();
    expect(components.DataTable).toBeDefined();
    expect(components.Card).toBeDefined();
    expect(components.EmptyState).toBeDefined();
    expect(components.KeyValue).toBeDefined();
  });
});
