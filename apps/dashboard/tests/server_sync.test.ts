import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  loadServerConfig,
  saveServerConfig,
  checkServerHealth,
  fetchProjects,
  pullSnapshot,
  snapshotToReportBundle,
  computeBaselineDiff,
  type SyncPullResponse,
} from "../src/lib/data/server.js";
import type { Finding, HealthScore } from "@elcode-forge/schema";

class MockLocalStorage {
  private store = new Map<string, string>();
  getItem(key: string) {
    return this.store.get(key) ?? null;
  }
  setItem(key: string, val: string) {
    this.store.set(key, val);
  }
  clear() {
    this.store.clear();
  }
}

describe("Server Config & LocalStorage Persistence", () => {
  let mockStorage: MockLocalStorage;

  beforeEach(() => {
    mockStorage = new MockLocalStorage();
    const g = globalThis as unknown as {
      window?: { localStorage?: MockLocalStorage };
      localStorage?: MockLocalStorage;
    };
    g.window = { localStorage: mockStorage };
    g.localStorage = mockStorage;
  });

  afterEach(() => {
    const g = globalThis as unknown as {
      window?: unknown;
      localStorage?: unknown;
    };
    delete g.window;
    delete g.localStorage;
  });

  it("loads default server configuration when localStorage is empty", () => {
    const config = loadServerConfig();
    expect(config.serverUrl).toBe("http://localhost:8080");
    expect(config.token).toBe("");
    expect(config.mode).toBe("local");
  });

  it("persists and reloads server configuration", () => {
    saveServerConfig({
      serverUrl: "https://forge.internal.company.com:8443",
      token: "secret_access_token_12345",
      mode: "cloud",
      activeRepoId: "core-service",
      activeBranch: "develop",
    });

    const reloaded = loadServerConfig();
    expect(reloaded.serverUrl).toBe("https://forge.internal.company.com:8443");
    expect(reloaded.token).toBe("secret_access_token_12345");
    expect(reloaded.mode).toBe("cloud");
    expect(reloaded.activeRepoId).toBe("core-service");
    expect(reloaded.activeBranch).toBe("develop");
  });
});

describe("Server Client API Methods", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("validates server health successfully", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: true,
      json: async () => ({ status: "ok", version: "1.0.0", database: "postgres" }),
    } as Response);

    const result = await checkServerHealth("http://localhost:8080", "test-token");
    expect(result.ok).toBe(true);
    expect(result.status).toBe("ok");
    expect(result.version).toBe("1.0.0");
  });

  it("handles server health check failure gracefully", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    } as Response);

    const result = await checkServerHealth("http://localhost:8080");
    expect(result.ok).toBe(false);
    expect(result.error).toContain("503");
  });

  it("fetches projects list from server", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: true,
      json: async () => [
        {
          repo_id: "elcode-core",
          name: "ELCODE Core",
          latest_branch: "main",
          latest_commit: "c12345678",
          latest_health: 92,
          findings_count: 3,
          updated_at: 1700000000,
        },
      ],
    } as Response);

    const result = await fetchProjects("http://localhost:8080", "test-token");
    expect(result.ok).toBe(true);
    expect(result.projects.length).toBe(1);
    expect(result.projects[0].repo_id).toBe("elcode-core");
    expect(result.projects[0].latest_health).toBe(92);
  });

  it("pulls snapshot and converts to valid ReportBundle", async () => {
    const mockFinding: Finding = {
      id: "F-001",
      category: "security",
      severity: "critical",
      confidence: "certain",
      title: "Hardcoded API key",
      description: "Credential exposed",
      evidence: "sk_live_12345",
      recommendation: "Rotate secret",
      rule_id: "sec/no-secret",
    };

    const mockPull: SyncPullResponse = {
      repo_id: "backend-api",
      branch: "main",
      baseline_commit_hash: "abcd1234ef",
      baseline_health_score: {
        overall: 88,
        dimensions: [
          { dimension: "architecture", score: 90, weight_pct: 35, reasons: [] },
          { dimension: "security", score: 85, weight_pct: 35, reasons: [] },
        ],
      },
      baseline_findings: [mockFinding],
      baseline_content_hash: "hash_xyz",
      sync_status: "up_to_date",
    };

    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: true,
      json: async () => mockPull,
    } as Response);

    const pullResult = await pullSnapshot("http://localhost:8080", "backend-api", "main");
    expect(pullResult.ok).toBe(true);
    expect(pullResult.snapshot).toBeDefined();

    if (pullResult.snapshot) {
      const bundle = snapshotToReportBundle(pullResult.snapshot, "Backend API");
      expect(bundle.repository.name).toBe("Backend API");
      expect(bundle.repository.findings.length).toBe(1);
      expect(bundle.repository.findings[0].id).toBe("F-001");
      expect(bundle.repository.health?.overall).toBe(88);
      expect(bundle.revision?.branch).toBe("main");
      expect(bundle.revision?.head_sha).toBe("abcd1234ef");
    }
  });
});

describe("PR Baseline Diff Calculation", () => {
  const findingBase1: Finding = {
    id: "F-BASE-1",
    category: "maintainability",
    severity: "medium",
    confidence: "high",
    title: "Long method",
    description: "Method exceeds 50 lines",
    evidence: "function run() { ... }",
    recommendation: "Split into smaller helpers",
    rule_id: "quality/max-file-lines",
  };

  const findingBase2: Finding = {
    id: "F-FIXED-1",
    category: "security",
    severity: "high",
    confidence: "high",
    title: "SQL injection vulnerability",
    description: "Unsanitized user input in query",
    evidence: "query(rawInput)",
    recommendation: "Use parameterized query",
    rule_id: "sec/sql-injection",
  };

  const findingPRNew: Finding = {
    id: "F-PR-NEW-1",
    category: "architecture",
    severity: "critical",
    confidence: "certain",
    title: "Direct DB access in UI component",
    description: "Importing database client directly in React component",
    evidence: "import { db } from '../db'",
    recommendation: "Use service layer",
    rule_id: "arch/no-direct-db-in-ui",
  };

  const baseHealth: HealthScore = {
    overall: 85,
    dimensions: [
      { dimension: "architecture", score: 85, weight_pct: 35, reasons: [] },
      { dimension: "security", score: 80, weight_pct: 35, reasons: [] },
      { dimension: "maintainability", score: 90, weight_pct: 30, reasons: [] },
    ],
  };

  const prHealthDegraded: HealthScore = {
    overall: 78,
    dimensions: [
      { dimension: "architecture", score: 70, weight_pct: 35, reasons: [] },
      { dimension: "security", score: 95, weight_pct: 35, reasons: [] },
      { dimension: "maintainability", score: 90, weight_pct: 30, reasons: [] },
    ],
  };

  it("calculates added, resolved, and persistent findings accurately", () => {
    const diff = computeBaselineDiff(
      "my-repo",
      {
        branch: "main",
        health: baseHealth,
        findings: [findingBase1, findingBase2],
      },
      {
        branch: "feat/feature-x",
        health: prHealthDegraded,
        findings: [findingBase1, findingPRNew], // findingBase2 was resolved, findingPRNew was added
      }
    );

    expect(diff.repo_id).toBe("my-repo");
    expect(diff.baseBranch).toBe("main");
    expect(diff.compareBranch).toBe("feat/feature-x");
    expect(diff.baseHealth).toBe(85);
    expect(diff.compareHealth).toBe(78);
    expect(diff.healthDelta).toBe(-7);
    expect(diff.trendDirection).toBe("degrading");

    // Added
    expect(diff.addedFindings.length).toBe(1);
    expect(diff.addedFindings[0].id).toBe("F-PR-NEW-1");

    // Resolved
    expect(diff.resolvedFindings.length).toBe(1);
    expect(diff.resolvedFindings[0].id).toBe("F-FIXED-1");

    // Unchanged
    expect(diff.unchangedFindings.length).toBe(1);
    expect(diff.unchangedFindings[0].id).toBe("F-BASE-1");

    // Dimension shift
    const archDelta = diff.dimensionDeltas.find(
      (d: { dimension: string; delta: number }) => d.dimension === "architecture"
    );
    expect(archDelta?.delta).toBe(-15);

    const secDelta = diff.dimensionDeltas.find(
      (d: { dimension: string; delta: number }) => d.dimension === "security"
    );
    expect(secDelta?.delta).toBe(+15);
  });

  it("reports improving trend when health score increases significantly", () => {
    const prHealthImproved: HealthScore = {
      overall: 95,
      dimensions: [
        { dimension: "architecture", score: 95, weight_pct: 35, reasons: [] },
        { dimension: "security", score: 95, weight_pct: 35, reasons: [] },
      ],
    };

    const diff = computeBaselineDiff(
      "my-repo",
      {
        branch: "main",
        health: baseHealth,
        findings: [findingBase1, findingBase2],
      },
      {
        branch: "feat/security-fixes",
        health: prHealthImproved,
        findings: [],
      }
    );

    expect(diff.healthDelta).toBe(10);
    expect(diff.trendDirection).toBe("improving");
    expect(diff.resolvedFindings.length).toBe(2);
    expect(diff.addedFindings.length).toBe(0);
  });
});
