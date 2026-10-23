/**
 * Forge Server Client & Baseline Diff Engine.
 * Provides client synchronization with `forge-server` and baseline comparison.
 */

import type { ReportBundle, Finding, HealthScore } from "@elcode-forge/schema";

export interface ServerConfig {
  serverUrl: string;
  token: string;
  mode: "local" | "cloud";
  activeRepoId?: string;
  activeBranch?: string;
}

export interface ProjectSummary {
  repo_id: string;
  name: string;
  latest_branch: string | null;
  latest_commit: string | null;
  latest_health: number | null;
  findings_count: number | null;
  updated_at: number;
}

export interface BranchSummary {
  branch: string;
  latest_commit: string;
  latest_health: number;
  findings_count: number;
  updated_at: number;
}

export interface HistoryPoint {
  commit_hash: string;
  branch: string;
  timestamp_secs: number;
  overall_health: number;
  findings_count: number;
  health_json: string;
}

export interface ProjectOverview {
  repo_id: string;
  name: string;
  branches_count: number;
  branches: BranchSummary[];
  latest_health: HealthScore | null;
  open_vulnerabilities_count: number;
  critical_vulnerabilities_count: number;
  recent_history: HistoryPoint[];
  updated_at: number;
}

export interface HistoryTrend {
  repo_id: string;
  branch: string;
  total_points: number;
  latest_health: number | null;
  average_health: number;
  min_health: number | null;
  max_health: number | null;
  trend_direction: "improving" | "degrading" | "stable";
  health_delta: number;
  architecture_avg: number;
  security_avg: number;
  maintainability_avg: number;
  points: HistoryPoint[];
}

export interface SyncPullResponse {
  repo_id: string;
  branch: string;
  baseline_commit_hash: string | null;
  baseline_health_score: HealthScore | null;
  baseline_findings: Finding[];
  baseline_content_hash: string | null;
  sync_status: string;
}

export interface BaselineDiffReport {
  repo_id: string;
  baseBranch: string;
  compareBranch: string;
  baseHealth: number;
  compareHealth: number;
  healthDelta: number;
  trendDirection: "improving" | "degrading" | "stable";
  addedFindings: Finding[];
  resolvedFindings: Finding[];
  unchangedFindings: Finding[];
  dimensionDeltas: {
    dimension: string;
    baseScore: number;
    compareScore: number;
    delta: number;
  }[];
}

const STORAGE_KEY = "eforge_server_config";

/**
 * Loads server configuration from browser localStorage.
 */
export function loadServerConfig(): ServerConfig {
  if (typeof window === "undefined" || !window.localStorage) {
    return {
      serverUrl: "http://localhost:8080",
      token: "",
      mode: "local",
    };
  }

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        serverUrl: parsed.serverUrl || "http://localhost:8080",
        token: parsed.token || "",
        mode: parsed.mode === "cloud" ? "cloud" : "local",
        activeRepoId: parsed.activeRepoId,
        activeBranch: parsed.activeBranch,
      };
    }
  } catch {
    // Fall back to default config if corrupted
  }

  return {
    serverUrl: "http://localhost:8080",
    token: "",
    mode: "local",
  };
}

/**
 * Persists server configuration into browser localStorage.
 */
export function saveServerConfig(cfg: ServerConfig): void {
  if (typeof window !== "undefined" && window.localStorage) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
    } catch {
      // Ignore quota errors in restricted contexts
    }
  }
}

/**
 * Creates default headers for Forge Server HTTP requests.
 */
function buildHeaders(token?: string): HeadersInit {
  const headers: Record<string, string> = {
    Accept: "application/json",
  };
  if (token && token.trim().length > 0) {
    headers["Authorization"] = `Bearer ${token.trim()}`;
  }
  return headers;
}

/**
 * Validates connection to the Forge Server via `/api/v1/health`.
 */
export async function checkServerHealth(
  serverUrl: string,
  token?: string
): Promise<{ ok: boolean; status?: string; version?: string; error?: string }> {
  try {
    const url = new URL("/api/v1/health", serverUrl).toString();
    const resp = await fetch(url, {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        error: `Server responded with status ${resp.status} ${resp.statusText}`,
      };
    }

    const data = await resp.json();
    return {
      ok: data.status === "ok",
      status: data.status,
      version: data.version,
    };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Failed to connect to Forge Server",
    };
  }
}

/**
 * Fetches all registered projects from the Forge Server.
 */
export async function fetchProjects(
  serverUrl: string,
  token?: string
): Promise<{ ok: boolean; projects: ProjectSummary[]; error?: string }> {
  try {
    const url = new URL("/api/v1/projects", serverUrl).toString();
    const resp = await fetch(url, {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        projects: [],
        error: `Failed to fetch projects (HTTP ${resp.status})`,
      };
    }

    const projects: ProjectSummary[] = await resp.json();
    return { ok: true, projects };
  } catch (err) {
    return {
      ok: false,
      projects: [],
      error: err instanceof Error ? err.message : "Network error fetching projects",
    };
  }
}

/**
 * Fetches detailed overview for a specific repository.
 */
export async function fetchProjectOverview(
  serverUrl: string,
  repoId: string,
  token?: string
): Promise<{ ok: boolean; overview?: ProjectOverview; error?: string }> {
  try {
    const url = new URL(
      `/api/v1/projects/${encodeURIComponent(repoId)}/overview`,
      serverUrl
    ).toString();
    const resp = await fetch(url, {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        error: `Project '${repoId}' overview not found (HTTP ${resp.status})`,
      };
    }

    const overview: ProjectOverview = await resp.json();
    return { ok: true, overview };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Network error fetching overview",
    };
  }
}

/**
 * Fetches distinct branches for a specific repository.
 */
export async function fetchProjectBranches(
  serverUrl: string,
  repoId: string,
  token?: string
): Promise<{ ok: boolean; branches: BranchSummary[]; error?: string }> {
  try {
    const url = new URL(
      `/api/v1/projects/${encodeURIComponent(repoId)}/branches`,
      serverUrl
    ).toString();
    const resp = await fetch(url, {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        branches: [],
        error: `Failed to fetch branches (HTTP ${resp.status})`,
      };
    }

    const branches: BranchSummary[] = await resp.json();
    return { ok: true, branches };
  } catch (err) {
    return {
      ok: false,
      branches: [],
      error: err instanceof Error ? err.message : "Network error fetching branches",
    };
  }
}

/**
 * Fetches historical score trend for a specific repository branch.
 */
export async function fetchProjectHistory(
  serverUrl: string,
  repoId: string,
  branch: string = "main",
  limit: number = 50,
  token?: string
): Promise<{ ok: boolean; trend?: HistoryTrend; error?: string }> {
  try {
    const url = new URL(
      `/api/v1/projects/${encodeURIComponent(repoId)}/history`,
      serverUrl
    );
    url.searchParams.set("branch", branch);
    url.searchParams.set("limit", limit.toString());

    const resp = await fetch(url.toString(), {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        error: `Failed to fetch history (HTTP ${resp.status})`,
      };
    }

    const trend: HistoryTrend = await resp.json();
    return { ok: true, trend };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Network error fetching history",
    };
  }
}

/**
 * Pulls a baseline snapshot from Forge Server via `/api/v1/sync/pull`.
 */
export async function pullSnapshot(
  serverUrl: string,
  repoId: string,
  branch: string = "main",
  token?: string
): Promise<{ ok: boolean; snapshot?: SyncPullResponse; error?: string }> {
  try {
    const url = new URL("/api/v1/sync/pull", serverUrl);
    url.searchParams.set("repo_id", repoId);
    url.searchParams.set("branch", branch);

    const resp = await fetch(url.toString(), {
      method: "GET",
      headers: buildHeaders(token),
    });

    if (!resp.ok) {
      return {
        ok: false,
        error: `Failed to pull snapshot (HTTP ${resp.status})`,
      };
    }

    const snapshot: SyncPullResponse = await resp.json();
    return { ok: true, snapshot };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Network error pulling snapshot",
    };
  }
}

/**
 * Converts a remote SyncPullResponse snapshot into a full ReportBundle for dashboard rendering.
 */
export function snapshotToReportBundle(
  snapshot: SyncPullResponse,
  repoName?: string
): ReportBundle {
  const health: HealthScore = snapshot.baseline_health_score || {
    overall: 100,
    dimensions: [
      { dimension: "architecture", score: 100, weight_pct: 35, reasons: [] },
      { dimension: "security", score: 100, weight_pct: 35, reasons: [] },
      { dimension: "maintainability", score: 100, weight_pct: 30, reasons: [] },
    ],
  };

  return {
    schema_version: 1,
    tool_version: "0.9.0",
    repository: {
      root_path: `/${snapshot.repo_id}`,
      name: repoName || snapshot.repo_id,
      detected_languages: ["TypeScript", "Rust"],
      detected_frameworks: [],
      package_managers: ["npm"],
      files: [],
      modules: [],
      dependencies: [],
      findings: snapshot.baseline_findings || [],
      health,
    },
    revision: {
      head_sha: snapshot.baseline_commit_hash || "00000000",
      branch: snapshot.branch,
      commit_unix_ts: Math.floor(Date.now() / 1000),
      dirty: false,
    },
    architecture: {
      circular_dependencies: [],
      high_centrality_modules: [],
    },
    hotspots: [],
    rules: [],
  };
}

/**
 * Computes a detailed differential comparison between a base branch and a PR branch.
 */
export function computeBaselineDiff(
  repoId: string,
  base: {
    branch: string;
    health: HealthScore;
    findings: Finding[];
  },
  compare: {
    branch: string;
    health: HealthScore;
    findings: Finding[];
  }
): BaselineDiffReport {
  const baseFindingIds = new Set(base.findings.map((f) => f.id));
  const compareFindingIds = new Set(compare.findings.map((f) => f.id));

  const addedFindings = compare.findings.filter((f) => !baseFindingIds.has(f.id));
  const resolvedFindings = base.findings.filter((f) => !compareFindingIds.has(f.id));
  const unchangedFindings = compare.findings.filter((f) => baseFindingIds.has(f.id));

  const baseHealth = base.health.overall;
  const compareHealth = compare.health.overall;
  const healthDelta = compareHealth - baseHealth;

  const trendDirection: "improving" | "degrading" | "stable" =
    healthDelta > 2 ? "improving" : healthDelta < -2 ? "degrading" : "stable";

  // Compare dimension scores
  const baseDims = new Map(base.health.dimensions.map((d) => [d.dimension.toLowerCase(), d.score]));
  const compareDims = new Map(compare.health.dimensions.map((d) => [d.dimension.toLowerCase(), d.score]));

  const allDimensions = Array.from(new Set([...baseDims.keys(), ...compareDims.keys()]));
  const dimensionDeltas = allDimensions.map((dim) => {
    const baseScore = baseDims.get(dim) ?? 100;
    const compareScore = compareDims.get(dim) ?? 100;
    return {
      dimension: dim,
      baseScore,
      compareScore,
      delta: compareScore - baseScore,
    };
  });

  return {
    repo_id: repoId,
    baseBranch: base.branch,
    compareBranch: compare.branch,
    baseHealth,
    compareHealth,
    healthDelta,
    trendDirection,
    addedFindings,
    resolvedFindings,
    unchangedFindings,
    dimensionDeltas,
  };
}
