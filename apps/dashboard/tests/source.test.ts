import { describe, expect, it } from "vitest";
import { loadFromApi } from "../src/lib/data/source.js";

describe("loadFromApi security boundaries", () => {
  it("rejects non-loopback URLs strictly", async () => {
    const res = await loadFromApi("https://evil-external-site.com/api");
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.errors[0]).toContain("must connect only to 127.0.0.1 or localhost");
    }
  });

  it("rejects public IP addresses", async () => {
    const res = await loadFromApi("http://8.8.8.8:7878");
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.errors[0]).toContain("must connect only to 127.0.0.1 or localhost");
    }
  });
});
