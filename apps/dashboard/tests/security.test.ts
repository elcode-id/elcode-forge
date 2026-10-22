import { describe, expect, it } from "vitest";
import type { Finding } from "@elcode-forge/schema";

describe("Security & vulnerability data helpers", () => {
  const mockVulnFinding: Finding = {
    id: "security/vulnerable-dependency-npm-lodash",
    category: "security",
    severity: "high",
    confidence: "certain",
    title: "Vulnerable dependency: lodash@4.17.15 (npm)",
    description: "Prototype Pollution in lodash allows malicious input to inject object properties.",
    evidence: "Vulnerable package: lodash@4.17.15 (npm) | Fixed in: 4.17.19 | References: CVE-2020-8203",
    recommendation: "Upgrade lodash to version 4.17.19 or higher.",
    rule_id: "security/vulnerable-dependency",
  };

  it("extracts CVE identifier from evidence", () => {
    const match = mockVulnFinding.evidence.match(/(CVE-\d{4}-\d{4,7}|GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4})/i);
    expect(match).not.toBeNull();
    expect(match?.[1]).toBe("CVE-2020-8203");
  });

  it("generates actionable package upgrade commands", () => {
    const ev = mockVulnFinding.evidence;
    const pkgMatch = ev.match(/Vulnerable package: ([^\s@]+)@([^\s]+) \((\w+)\)/);
    const fixedMatch = ev.match(/Fixed in: ([^\s|]+)/);

    expect(pkgMatch).not.toBeNull();
    expect(fixedMatch).not.toBeNull();

    const [, pkgName, , eco] = pkgMatch!;
    const fixedVer = fixedMatch![1];
    expect(pkgName).toBe("lodash");
    expect(fixedVer).toBe("4.17.19");
    expect(eco).toBe("npm");

    const cmd = `npm install ${pkgName}@^${fixedVer}`;
    expect(cmd).toBe("npm install lodash@^4.17.19");
  });
});
