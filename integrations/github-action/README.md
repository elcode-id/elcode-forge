# ELCODE Forge GitHub Action (`elcode-id/forge-action`)

Run deterministic repository health checks, blast-radius pull request reviews, dependency hygiene audits, and SARIF v2.1.0 security scans in GitHub Actions using **`eforge`**.

## Quick Start

### 1. Pull Request Blast-Radius & Risk Review

```yaml
name: ELCODE Forge PR Review

on:
  pull_request:
    branches: [main, develop]

jobs:
  forge-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Run ELCODE Forge PR Review
        uses: ./integrations/github-action
        with:
          command: review
          base: origin/${{ github.base_ref }}
          strict: "true"
          step-summary: "true"
```

### 2. Full Repository Scan + SARIF Export

```yaml
      - name: Run ELCODE Forge Security Scan
        uses: ./integrations/github-action
        with:
          command: scan
          fail-on: high
          sarif-output: eforge-results.sarif
```

## Inputs

| Input | Default | Description |
| :--- | :--- | :--- |
| `command` | `review` | `eforge` subcommand (`doctor`, `scan`, `review`, `diff`, `security`, `dependencies`, `graph`). |
| `path` | `.` | Target repository directory to analyze. |
| `base` | `HEAD~1` | Base Git revision for `review` and `diff` subcommands. |
| `format` | `terminal` | Output format (`terminal`, `json`, `sarif`, `markdown`). |
| `strict` | `false` | Fail with exit code `2` on policy violations (`high`/`critical` findings). |
| `fail-on` | `""` | Custom minimum severity threshold (`critical`, `high`, `medium`, `low`). |
| `sarif-output` | `""` | Optional file path to emit SARIF v2.1.0 report (`scan` / `security`). |
| `step-summary` | `true` | Append Markdown report to `$GITHUB_STEP_SUMMARY`. |

## Outputs

| Output | Description |
| :--- | :--- |
| `exit-code` | Exit code returned by `eforge` (`0` = pass, `2` = policy gate violation). |
| `sarif-file` | Path to generated SARIF report file when `sarif-output` is specified. |
