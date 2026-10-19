import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { loadHistory } from "../src/lib/data/history.js";

describe("History client", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("rejects non-loopback URLs for security boundary enforcement", async () => {
    await expect(loadHistory("https://example.com")).rejects.toThrow(
      /Security rejection/
    );
    await expect(loadHistory("http://192.168.1.5:7878")).rejects.toThrow(
      /Security rejection/
    );
  });

  it("fetches history snapshots successfully from local loopback", async () => {
    const mockSnapshots = [
      {
        head_sha: "abc123456789",
        commit_unix_ts: 1700000000,
        overall: 85,
        dimensions: { architecture: 80, security: 90 },
        findings_by_severity: { critical: 0, high: 1, medium: 2, low: 0, info: 0 },
      },
    ];

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        limit: 30,
        total: 1,
        snapshots: mockSnapshots,
      }),
    } as unknown as Response);

    const result = await loadHistory("http://127.0.0.1:7878", 30);
    expect(result).toEqual(mockSnapshots);
    expect(globalThis.fetch).toHaveBeenCalledWith(
      "http://127.0.0.1:7878/api/v1/history?limit=30",
      expect.objectContaining({ method: "GET" })
    );
  });

  it("throws actionable error on server error HTTP 500", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
    } as unknown as Response);

    await expect(loadHistory("http://localhost:7878")).rejects.toThrow(
      /HTTP 500/
    );
  });
});
