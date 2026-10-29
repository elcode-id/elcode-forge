<svelte:head>
  <title>Security, Supply-Chain & Compliance Audits | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8">
  <p class="lead">
    Execute local-first OSV vulnerability audits, Shannon entropy secret detection, polyglot unsafe code scanning, CycloneDX/SPDX SBOM generation, open-source license compliance checks, and automated SOC 2 Type II, OWASP Top 10 (2021), and ISO/IEC 27001:2022 compliance reports.
  </p>

  <!-- 1. Automated Compliance Audits (eforge audit) -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-6">
    <div>
      <h2 class="text-lg font-bold text-white font-mono">1. Automated Compliance Audits (<code class="text-purple-300">eforge audit</code>)</h2>
      <p class="text-sm text-slate-300 mt-1 leading-relaxed">
        Maps deterministic repository findings, dependency health, and architectural metrics to formal compliance clauses. Returns <code class="text-emerald-400 font-mono">ExitCode::Success (0)</code> when all clauses pass or <code class="text-purple-400 font-mono">ExitCode::SecurityPolicyFailure (4)</code> if any clause fails.
      </p>
    </div>

    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-200 overflow-x-auto">
      <pre><code>{`# SOC 2 Type II Trust Services Criteria audit
eforge audit --standard soc2 --format terminal

# OWASP Top 10 (2021) audit exported as GitHub-Flavored Markdown
eforge audit --standard owasp --format markdown -o owasp-audit.md

# ISO/IEC 27001:2022 Information Security Controls exported as JSON
eforge audit --standard iso27001 --format json -o iso27001-audit.json`}</code></pre>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <div class="bg-black/40 border border-slate-800 rounded-xl p-4 space-y-2.5">
        <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-blue-400"></span>
          <span>SOC 2 Type II (--standard soc2)</span>
        </div>
        <ul class="text-xs text-slate-400 space-y-2 font-mono">
          <li>• <strong class="text-slate-200">CC6.1 (Logical Access):</strong> Zero hardcoded secrets, private keys, or high-entropy tokens.</li>
          <li>• <strong class="text-slate-200">CC6.6 (Boundary Protection):</strong> Layer boundary rules (<code class="text-slate-300">arch/*</code>) and CORS hygiene.</li>
          <li>• <strong class="text-slate-200">CC7.1 (Vulnerability Mgmt):</strong> Zero unpatched OSV dependency CVEs or unsafe code patterns.</li>
          <li>• <strong class="text-slate-200">CC8.1 (Change Management):</strong> Acyclic architecture graph, companion tests, and lockfile hygiene.</li>
        </ul>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-xl p-4 space-y-2.5">
        <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-purple-400"></span>
          <span>OWASP Top 10 (--standard owasp)</span>
        </div>
        <ul class="text-xs text-slate-400 space-y-2 font-mono">
          <li>• <strong class="text-slate-200">A01 (Broken Access Control):</strong> Enforces architectural layer isolation (<code class="text-slate-300">no-direct-db-in-ui</code>) and <code class="text-slate-300">permissive-cors</code> checks.</li>
          <li>• <strong class="text-slate-200">A02 (Cryptographic Failures):</strong> Verifies absence of leaked credentials, API tokens, and private keys.</li>
          <li>• <strong class="text-slate-200">A03 (Injection):</strong> Scans for <code class="text-slate-300">eval</code>, shell exec, <code class="text-slate-300">dangerouslySetInnerHTML</code>, <code class="text-slate-300">pickle.loads</code>, and <code class="text-slate-300">unserialize</code>.</li>
          <li>• <strong class="text-slate-200">A06 (Vulnerable Components):</strong> Audits all dependencies against OSV/GHSA advisories and unpinned versions.</li>
        </ul>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-xl p-4 space-y-2.5">
        <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>ISO/IEC 27001 (--standard iso27001)</span>
        </div>
        <ul class="text-xs text-slate-400 space-y-2 font-mono">
          <li>• <strong class="text-slate-200">Control 8.8 (Technical Vulnerabilities):</strong> Continuous polyglot dependency CVE scanning (<code class="text-slate-300">security/vulnerable-dependency</code>) and lockfile pinning.</li>
          <li>• <strong class="text-slate-200">Control 8.28 (Secure Coding):</strong> Enforces static security pattern rules, secret redaction, and architectural modularity.</li>
        </ul>
      </div>
    </div>
  </div>

  <!-- 2. OSV Vulnerability Auditing & SBOM Generation -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
      <h2 class="text-lg font-bold text-white font-mono">2. OSV Vulnerability Audit (<code class="text-rose-400">eforge security --audit</code>)</h2>
      <p class="text-xs text-slate-300 leading-relaxed">
        Audits declared and locked dependencies across <strong>npm</strong> (<code class="text-slate-200">pkg:npm/</code>), <strong>PyPI</strong> (<code class="text-slate-200">pkg:pypi/</code>), <strong>Go</strong> (<code class="text-slate-200">pkg:golang/</code>), <strong>Cargo</strong> (<code class="text-slate-200">pkg:cargo/</code>), and <strong>Composer</strong> (<code class="text-slate-200">pkg:composer/</code>) against OSV and GHSA advisories, with offline cache support in <code class="text-slate-200">.eforge/cache/osv/</code>:
      </p>
      <div class="bg-black/60 border border-slate-800 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
        <pre><code>{`# Deep dependency CVE audit + secret & pattern scan
eforge security --audit --fail-on high

# Strictly offline audit using cached & embedded advisories
eforge security --audit --offline

# Export OASIS SARIF v2.1.0 with CVSS v3 scores
eforge security --audit --format sarif -o security.sarif`}</code></pre>
      </div>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
      <h2 class="text-lg font-bold text-white font-mono">3. NTIA-Compliant SBOM (<code class="text-emerald-400">eforge sbom</code>)</h2>
      <p class="text-xs text-slate-300 leading-relaxed">
        Generates deterministic Software Bill of Materials in <strong>OASIS CycloneDX v1.5 JSON</strong> (<code class="text-slate-200">--format cyclonedx</code>) or <strong>ISO/IEC 5962:2021 SPDX v2.3 JSON</strong> (<code class="text-slate-200">--format spdx</code>) with canonical Package URLs (PURL), license categories, and BLAKE3 serial numbers:
      </p>
      <div class="bg-black/60 border border-slate-800 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
        <pre><code>{`# Export CycloneDX v1.5 JSON SBOM
eforge sbom --format cyclonedx -o sbom.cdx.json

# Export SPDX v2.3 JSON SBOM
eforge sbom --format spdx -o sbom.spdx.json`}</code></pre>
      </div>
    </div>
  </div>

  <!-- 4. Secret Scanner, Shannon Entropy & Polyglot Unsafe Code Rules -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">4. Secret Detection, Shannon Entropy & Built-in Security Rules</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Every discovered credential is strictly masked with <code class="text-rose-400 font-mono">****REDACTED****</code> in finding evidence and <code class="text-rose-400 font-mono">[REDACTED_SECRET]</code> in AI context packs. Candidate tokens with length &ge; <code class="text-slate-200 font-mono">entropy_min_len</code> (20) and Shannon entropy &ge; <code class="text-slate-200 font-mono">entropy_threshold</code> (4.5 bits/symbol, proportionally scaled for hex strings) trigger <code class="text-amber-300 font-mono">security/high-entropy-secret</code>.
    </p>

    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2 pr-4">Rule ID</th>
            <th class="py-2 px-3">Ecosystem</th>
            <th class="py-2 px-3">Severity</th>
            <th class="py-2 pl-3">Detection Target</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          <tr>
            <td class="py-2 pr-4 text-rose-400 font-bold">security/hardcoded-secret</td>
            <td class="py-2 px-3">All</td>
            <td class="py-2 px-3 text-rose-400">Critical</td>
            <td class="py-2 pl-3 font-sans">Verified AWS (<code class="font-mono">AKIA...</code>), GitHub PATs (<code class="font-mono">ghp_</code>, <code class="font-mono">github_pat_</code>), Slack (<code class="font-mono">xoxb-</code>, <code class="font-mono">xoxp-</code>), OpenAI (<code class="font-mono">sk-</code>), Anthropic (<code class="font-mono">sk-ant-</code>), and generic secret assignments.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-rose-400 font-bold">security/private-key</td>
            <td class="py-2 px-3">All</td>
            <td class="py-2 px-3 text-rose-400">Critical</td>
            <td class="py-2 pl-3 font-sans">PEM RSA, EC, DSA, OpenSSH, and PKCS#8 private key blocks.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-amber-400 font-bold">security/high-entropy-secret</td>
            <td class="py-2 px-3">All</td>
            <td class="py-2 px-3 text-amber-400">High</td>
            <td class="py-2 pl-3 font-sans">Unclassified high-entropy tokens (&ge; 4.5 bits/symbol, length &ge; 20, lockfiles & allowlist excluded).</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-blue-300 font-bold">security/no-eval, no-implied-eval, no-dangerous-inner-html, unsafe-shell-exec, permissive-cors</td>
            <td class="py-2 px-3">JS / TS / SFC</td>
            <td class="py-2 px-3 text-amber-400">Critical / High</td>
            <td class="py-2 pl-3 font-sans">Dynamic <code class="font-mono">eval()</code>, <code class="font-mono">new Function()</code>, string <code class="font-mono">setTimeout</code>, <code class="font-mono">dangerouslySetInnerHTML</code>, <code class="font-mono">child_process.exec</code>, and wildcard CORS.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-emerald-300 font-bold">security/python-unsafe-eval, python-insecure-subprocess, python-pickle-deserialization</td>
            <td class="py-2 px-3">Python</td>
            <td class="py-2 px-3 text-amber-400">Critical / High</td>
            <td class="py-2 pl-3 font-sans">Python <code class="font-mono">eval()</code>/<code class="font-mono">exec()</code>, <code class="font-mono">subprocess(..., shell=True)</code>/<code class="font-mono">os.system()</code>, and untrusted <code class="font-mono">pickle.loads()</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-purple-300 font-bold">security/php-unsafe-eval, php-insecure-unserialize, php-command-injection, php-create-function</td>
            <td class="py-2 px-3">PHP</td>
            <td class="py-2 px-3 text-amber-400">Critical / High</td>
            <td class="py-2 pl-3 font-sans">PHP <code class="font-mono">eval()</code>, <code class="font-mono">unserialize()</code> object injection, <code class="font-mono">shell_exec</code>/<code class="font-mono">system</code>/<code class="font-mono">passthru</code>/<code class="font-mono">proc_open</code>, and <code class="font-mono">create_function()</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">license/disallowed-copyleft, denied-license, unapproved-license, missing-project-license</td>
            <td class="py-2 px-3">All</td>
            <td class="py-2 px-3 text-amber-400">High / Med / Low</td>
            <td class="py-2 pl-3 font-sans">Enforces <code class="font-mono">license.disallow_copyleft</code>, <code class="font-mono">license.denylist</code>, <code class="font-mono">license.allowlist</code>, and root <code class="font-mono">LICENSE</code> presence.</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</div>
