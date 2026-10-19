export interface HistorySnapshot {
  head_sha: string;
  commit_unix_ts: number;
  overall: number;
  dimensions: Record<string, number>;
  findings_by_severity: Record<string, number>;
}

export interface HistoryApiResponse {
  limit: number;
  total: number;
  snapshots: HistorySnapshot[];
}

/**
 * Validates that an API URL targets local loopback only.
 */
function isLoopback(urlStr: string): boolean {
  try {
    const parsed = new URL(urlStr);
    const host = parsed.hostname.toLowerCase();
    return (
      host === "127.0.0.1" ||
      host === "localhost" ||
      host === "::1" ||
      host === "[::1]"
    );
  } catch {
    return false;
  }
}

/**
 * Loads deterministic history snapshots from local eforge serve instance.
 */
export async function loadHistory(
  baseUrl: string,
  limit: number = 30
): Promise<HistorySnapshot[]> {
  if (!isLoopback(baseUrl)) {
    throw new Error(
      `Security rejection: Base URL '${baseUrl}' does not target local loopback (127.0.0.1 or localhost)`
    );
  }

  const cleanBase = baseUrl.replace(/\/+$/, "");
  const targetUrl = `${cleanBase}/api/v1/history?limit=${Math.max(1, limit)}`;

  const res = await fetch(targetUrl, {
    method: "GET",
    headers: {
      Accept: "application/json",
    },
  });

  if (!res.ok) {
    throw new Error(`Failed to fetch history snapshots: HTTP ${res.status}`);
  }

  const data = (await res.json()) as HistoryApiResponse;
  if (!data || !Array.isArray(data.snapshots)) {
    return [];
  }

  return data.snapshots;
}
