import { parseReportBundle, type ReportBundle } from "@elcode-forge/schema";

export type DataSource =
  | { kind: "file"; name: string }
  | { kind: "live"; baseUrl: string }
  | { kind: "server"; serverUrl: string; repoId: string; branch: string };

export type LoadResult =
  | { ok: true; bundle: ReportBundle; source: DataSource }
  | { ok: false; errors: string[] };

/**
 * Loads and validates a ReportBundle from a user-supplied local JSON File.
 */
export async function loadFromFile(file: File): Promise<LoadResult> {
  try {
    const text = await file.text();
    const parsed = JSON.parse(text);
    const validated = parseReportBundle(parsed);
    if (!validated.ok) {
      return { ok: false, errors: validated.errors };
    }
    return {
      ok: true,
      bundle: validated.bundle,
      source: { kind: "file", name: file.name },
    };
  } catch (err) {
    return {
      ok: false,
      errors: [err instanceof Error ? err.message : "Failed to read bundle file"],
    };
  }
}

/**
 * Loads and validates a ReportBundle from a local `eforge serve` HTTP API.
 * Restricts URLs strictly to loopback addresses (127.0.0.1 or localhost).
 */
export async function loadFromApi(baseUrl: string): Promise<LoadResult> {
  try {
    const url = new URL(baseUrl);
    const isLoopback =
      url.hostname === "127.0.0.1" ||
      url.hostname === "localhost" ||
      url.hostname === "::1";

    if (!isLoopback) {
      return {
        ok: false,
        errors: ["Security violation: live API must connect only to 127.0.0.1 or localhost"],
      };
    }

    const endpoint = new URL("/api/v1/bundle", url).toString();
    const resp = await fetch(endpoint, {
      method: "GET",
      headers: { Accept: "application/json" },
    });

    if (!resp.ok) {
      return {
        ok: false,
        errors: [`HTTP ${resp.status} ${resp.statusText} from ${endpoint}`],
      };
    }

    const json = await resp.json();
    const validated = parseReportBundle(json);
    if (!validated.ok) {
      return { ok: false, errors: validated.errors };
    }

    return {
      ok: true,
      bundle: validated.bundle,
      source: { kind: "live", baseUrl: url.origin },
    };
  } catch (err) {
    return {
      ok: false,
      errors: [err instanceof Error ? err.message : "Network error contacting eforge serve API"],
    };
  }
}
