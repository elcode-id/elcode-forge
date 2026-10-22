# ELCODE Forge Security Guide — Local-First Intelligence & Supply-Chain Durability

> *"Software should feel like an instrument, not a compromise."*  
> *"We use AI as leverage, not as identity. Deterministic before generative."*  
> — [ELCODE Studio Manifesto](https://elcode.id/)

ELCODE Forge (`eforge`) provides enterprise-grade, deterministic, and local-first software security analysis. Designed for zero-panic durability and sub-second execution, `eforge` ensures that your proprietary source code, package manifests, and Git history never leave your machine during security evaluations.

---

## Table of Contents
1. [Security Architecture & Principles](#1-security-architecture--principles)
2. [Vulnerability Auditing & OSV Intelligence (`eforge security --audit`)](#2-vulnerability-auditing--osv-intelligence)
3. [Software Bill of Materials (SBOM) (`eforge sbom`)](#3-software-bill-of-materials-sbom)
4. [License Compliance & Copyleft Safeguards](#4-license-compliance--copyleft-safeguards)
5. [Credential Detection & Shannon Entropy Scanner](#5-credential-detection--shannon-entropy-scanner)
6. [Security Configuration (`eforge.config.yaml`)](#6-security-configuration)
7. [CI/CD Gates & GitHub Code Scanning (SARIF)](#7-cicd-gates--github-code-scanning-sarif)

---

## 1. Security Architecture & Principles

- **Zero Cloud Exfiltration:** Vulnerability matching, secret scanning, and license detection execute 100% locally on your machine or CI runner.
- **Deterministic & Explainable:** Identical repository state and configuration always generate byte-for-byte identical findings, risk scores, and SBOM documents.
- **Mandatory Redaction Guarantee:** Discovered credentials, API tokens, and private keys are masked (`****REDACTED****`) before being persisted to reports, JSON, SARIF, or AI prompt contexts.
- **Untrusted Input Boundary:** All repository paths, manifests, and Git histories are treated as untrusted input with canonical path boundary guards preventing directory traversal escapes (`../`).

---

## 2. Vulnerability Auditing & OSV Intelligence

`eforge security --audit` audits multi-ecosystem dependencies against the Open Source Vulnerabilities (OSV) database and GitHub Advisory Database (GHSA).

### Supported Ecosystems
| Ecosystem | Manifests Analyzed | Lockfiles Parsed | PURL Namespace |
| :--- | :--- | :--- | :--- |
| **npm** | `package.json` | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock` | `pkg:npm/<name>` |
| **Cargo** | `Cargo.toml` | `Cargo.lock` | `pkg:cargo/<name>` |
| **PyPI** | `requirements.txt`, `pyproject.toml` | `poetry.lock`, `Pipfile.lock` | `pkg:pypi/<name>` |
| **Go** | `go.mod` | `go.sum` | `pkg:golang/<name>` |
| **Composer** | `composer.json` | `composer.lock` | `pkg:composer/<name>` |

### Usage
```bash
# Focused source code secret and unsafe pattern scan
eforge security

# Full dependency vulnerability audit across all ecosystems
eforge security --audit

# Fail CI pipeline if High or Critical vulnerabilities are discovered
eforge security --audit --fail-on high

# Export OASIS SARIF v2.1.0 for GitHub Security tab
eforge security --audit --format sarif -o security.sarif
```

---

## 3. Software Bill of Materials (SBOM)

`eforge sbom` generates compliant, deterministic Software Bill of Materials in standard formats for software supply chain transparency (NTIA & Executive Order 14028 compliant).

### Supported Formats
- **CycloneDX v1.5 JSON** (`--format cyclonedx`): Native component manifests with Package URLs (PURL), license categories, and deterministic BLAKE3 serial numbers (`urn:uuid:...`).
- **SPDX v2.3 JSON** (`--format spdx`): Linux Foundation standard with component packages, external references, and SPDX license identifiers.

### Usage
```bash
# Output CycloneDX v1.5 JSON to stdout
eforge sbom --format cyclonedx

# Export CycloneDX SBOM to a file
eforge sbom --format cyclonedx -o sbom.cdx.json

# Export SPDX v2.3 JSON to a file
eforge sbom --format spdx -o sbom.spdx.json
```

---

## 4. License Compliance & Copyleft Safeguards

ELCODE Forge protects proprietary codebases from unintentional open-source license contamination (such as strict copyleft viral obligations under GPL-2.0, GPL-3.0, or AGPL-3.0).

### Classification Tiers
1. **Permissive:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, CC0-1.0, Unlicense.
2. **Weak Copyleft:** LGPL-2.1, LGPL-3.0, MPL-2.0, EPL-2.0, CDDL-1.0.
3. **Strong Copyleft:** GPL-2.0, GPL-3.0, AGPL-3.0, OSL-3.0, SSPL-1.0.
4. **Proprietary / Unknown:** Custom commercial licenses or undeclared license terms.

### Rules Evaluated
- `license/disallowed-copyleft` (High) — Triggered when `disallow_copyleft: true` and a strong copyleft package is introduced.
- `license/denied-license` (High) — Triggered when a dependency matches an entry in `license.denylist`.
- `license/unapproved-license` (Medium) — Triggered when `license.allowlist` is defined and a dependency is not in the list.
- `license/missing-project-license` (Low) — Triggered when the root repository lacks a recognized `LICENSE` file or manifest declaration.

---

## 5. Credential Detection & Shannon Entropy Scanner

`eforge` combines high-fidelity pattern matching for known cloud credentials with a Shannon entropy engine to detect generic high-entropy API secrets.

### Verified Credential Patterns
- **AWS Access Keys:** `AKIA[0-9A-Z]{16}` (Severity: `Critical`)
- **GitHub Personal Access Tokens:** Classic `ghp_[0-9a-zA-Z]{36}` and Fine-Grained `github_pat_[0-9a-zA-Z_]{82}` (Severity: `Critical`)
- **Slack Tokens:** Bot `xoxb-...` and User `xoxp-...` (Severity: `Critical`)
- **OpenAI API Keys:** Standard `sk-[a-zA-Z0-9]{48}` and Project `sk-proj-[a-zA-Z0-9_-]{48,}` (Severity: `Critical`)
- **Anthropic API Keys:** `sk-ant-[a-zA-Z0-9_-]{40,}` (Severity: `Critical`)
- **PEM Private Key Material:** `-----BEGIN ... PRIVATE KEY-----` (Severity: `Critical`)

### Shannon Entropy Engine
For custom or unclassified secret strings (e.g. database credentials, JWT secrets, encryption keys), `eforge` calculates mathematical Shannon entropy:
\[
  H(X) = -\sum_{x \in X} P(x) \log_2 P(x)
\]
- **Default Threshold:** Candidate tokens with length $\ge 20$ and entropy $\ge 4.5$ bits/symbol trigger `security/high-entropy-secret` (`Severity::High`).
- **Proportional Hex Scaling:** For pure hexadecimal strings (max theoretical entropy $\log_2(16) = 4.0$), the threshold is proportionally scaled to prevent false positives on sha256 checksums while detecting true 256-bit secrets.

---

## 6. Security Configuration

Configure security behavior in your root `eforge.config.yaml`:

```yaml
# eforge.config.yaml
version: 1
project_name: "my-service"

# Security & Secret Scanning Policy
security:
  # Paths, folders, and globs excluded from secret and security scanning
  allowlist:
    - "fixtures/**"
    - "tests/**"
    - "**/tests/**"
    - "packaging/**"
  # Shannon entropy threshold for candidate secret tokens (default: 4.5)
  entropy_threshold: 4.5
  # Minimum token length for entropy evaluation (default: 20)
  entropy_min_len: 20

# Open-Source License Compliance Policy
license:
  # Disallow strong copyleft licenses (GPL, AGPL)
  disallow_copyleft: true
  # Explicitly approved licenses
  allowlist:
    - "MIT"
    - "Apache-2.0"
    - "BSD-3-Clause"
    - "ISC"
  # Explicitly forbidden licenses
  denylist:
    - "AGPL-3.0"
    - "SSPL-1.0"

# Fail policy gates on high or critical findings
fail_on_severity: "high"
```

---

## 7. CI/CD Gates & GitHub Code Scanning (SARIF)

Integrate ELCODE Forge into your GitHub Actions workflow for automated pull request security gates and SARIF uploads:

```yaml
name: Security & Supply-Chain Gate

on:
  pull_request:
    branches: [develop, main]
  push:
    branches: [main]

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install ELCODE Forge
        run: cargo install --path crates/forge-cli --locked

      # 1. Run local-first security audit with SARIF output
      - name: Vulnerability & Secret Audit
        run: eforge security --audit --format sarif -o results.sarif --fail-on high

      # 2. Upload SARIF to GitHub Code Scanning tab
      - name: Upload SARIF report
        uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: results.sarif

      # 3. Generate CycloneDX SBOM artifact
      - name: Generate CycloneDX SBOM
        run: eforge sbom --format cyclonedx -o cyclonedx.sbom.json

      - name: Archive SBOM Artifact
        uses: actions/upload-artifact@v4
        with:
          name: sbom-cyclonedx
          path: cyclonedx.sbom.json
```
