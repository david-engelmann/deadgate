# Deadgate Operator Guide

Deadgate asks whether it ran. Given three offline local exports (branch protection settings, `.github/workflows` YAML files, and job execution history), Deadgate reconciles every required status check for a target branch against the workflow jobs that can produce it and the run records left behind in CI history, reporting which checks are genuinely `alive` and which exist only on paper (`no_producer`, `never_run`, or `skipped_only`).

## Installation

### Building from Source

With a Rust 1.80 or newer toolchain installed, build and install directly from the repository (works today on every tagged revision).

```sh
cargo install --git https://github.com/david-engelmann/deadgate --locked
```

### Prebuilt Release Archive

For releases tagged from `v0.25.0` onward on Linux (`x86_64`, `aarch64`) and macOS (`aarch64`, `x86_64`), the installer downloads the matching GitHub Release archive, verifies its SHA-256 digest, and installs `deadgate` into `$HOME/.local/bin` (or `$DEADGATE_INSTALL_DIR`).

```sh
curl -fsSL https://raw.githubusercontent.com/david-engelmann/deadgate/main/install.sh | sh
```

Pin a specific release tag by setting `DEADGATE_VERSION` (for example `DEADGATE_VERSION=v0.25.0`). Because `install.sh` does not edit shell profile files, ensure `$HOME/.local/bin` (or your custom `$DEADGATE_INSTALL_DIR`) stands on `PATH` in your shell (`export PATH="$HOME/.local/bin:$PATH"`). On Windows (`x86_64-pc-windows-msvc`), download the `.zip` archive and its `.sha256` file from the GitHub Releases page from `v0.25.0` onward.

## Offline Architecture and Exporting GitHub Inputs

Deadgate intentionally makes zero network calls and requires zero API tokens. All three inputs are local files on disk so security and compliance teams can audit sensitive repositories from air-gapped or read-only snapshots.

### 1. Exporting Branch Protection (`--protection`)

Deadgate reads classic GitHub branch protection JSON containing `required_status_checks.checks` (list of `{"context": "..."}` objects) or legacy `required_status_checks.contexts` (list of context strings). Duplicate contexts are deduplicated in first-seen order, and strict JSON parsing refuses duplicate object keys at any nesting level.

```sh
gh api "repos/owner/repo/branches/main/protection" > protection.json
```

### 2. Workflow Folder (`--workflows`)

Point `--workflows` at your repository's `.github/workflows` directory. Deadgate reads every `.yml` and `.yaml` file in filename order with its PyYAML-arbitrated YAML 1.1 reader, extracting workflow names (falling back to filename stem), `on` trigger blocks (`push` and `pull_request` branch filters and `paths` / `paths-ignore` filters), job identifiers, display `name` fields, `if` conditions, and `strategy.matrix` combinations (`include` and `exclude` rules included).

### 3. Exporting Job Execution History (`--history`)

Deadgate expects a JSON object carrying a top-level `"jobs"` list (`{"jobs": [...]}`). Each job record is an object carrying

- `name` Non-empty job run name as emitted by GitHub Actions (for example `"build"` or `"test (ubuntu-latest, 3.12)"`).
- `workflow_name` Workflow display name matching the producing workflow's `name` (or filename stem).
- `status` Run status string such as `"completed"`, `"in_progress"`, or `"queued"`.
- `conclusion` Run conclusion string (`"success"`, `"failure"`, `"cancelled"`, `"timed_out"`, `"neutral"`, `"skipped"`, or `null`).
- `started_at` and `completed_at` ISO 8601 timestamps (at least one timestamp is required whenever `status` is `"completed"`).

Because GitHub's REST API separates workflow runs (`/actions/runs`) from per-run job listings (`/actions/runs/{run_id}/jobs`), aggregate recent job records on your target branch with `gh` and `jq`.

```sh
(
  set -eu
  REPO="owner/repo"
  BRANCH="main"

  gh api "repos/$REPO/actions/runs?branch=$BRANCH&per_page=30" \
    --jq '.workflow_runs[] | "\(.id)\t\(.name)"' > runs.tsv

  cp /dev/null jobs.jsonl
  while IFS="$(printf '\t')" read -r RUN_ID WF_NAME; do
    gh api --paginate "repos/$REPO/actions/runs/$RUN_ID/jobs?per_page=100" > run-jobs.json
    jq -c --arg wf "$WF_NAME" '.jobs[] | .workflow_name = (.workflow_name // $wf)' run-jobs.json >> jobs.jsonl
  done < runs.tsv
  rm -f runs.tsv run-jobs.json
  jq -s '{jobs: .}' jobs.jsonl > history.json
  rm -f jobs.jsonl
)
```

## Command Line Interface

```
deadgate [-h | --help] --protection PROTECTION --workflows WORKFLOWS
         --history HISTORY [--branch BRANCH] [--markdown]
```

Options

- `-h`, `--help` Print help text to standard output and exit `0`.
- `--protection PROTECTION` Path to the branch protection JSON export.
- `--workflows WORKFLOWS` Path to the directory containing workflow `.yml` / `.yaml` definitions.
- `--history HISTORY` Path to the execution history JSON export.
- `--branch BRANCH` Branch covered by the protection export (defaults to `main`).
- `--markdown` Render a Markdown table instead of the default `deadgate-report/1` JSON document.

Exit codes

- `0` Reconciliation succeeded and report was written to standard output (or `--help` printed). Check `summary.paper` in the JSON output to fail CI builds when paper checks are present.
- `2` Usage error, unreadable input file, malformed JSON/YAML, duplicate JSON key, invalid filter pattern, or missing required status checks.

## How Verdicts and Notes Are Assigned

For each required status check context in `--protection`, Deadgate finds every job in workflows whose `push` or `pull_request` trigger admits `--branch` (matching both bare job form `<job>` and workflow-qualified form `<workflow> / <job>`, expanded across exact `strategy.matrix` combinations) and classifies the check into one of four statuses

- `alive` At least one matching producer exists for `--branch` and at least one completed history record from a producing workflow finished with `success`, `failure`, `cancelled`, `timed_out`, or `neutral`. `last_run_date` records the latest UTC date (`YYYY-MM-DD`).
- `skipped_only` At least one producer exists, zero records ran to a non-skipped conclusion, and at least one completed record finished with `"skipped"` as its `conclusion`.
- `never_run` At least one producer exists, and zero completed history records ran or skipped for it within the history window.
- `no_producer` Zero jobs in workflows admitting `--branch` can produce the required context. Deadgate searches loose historical records matching the bare or full context name so a renamed check still shows the stale `last_run_date` when it last ran under its old name.

Diagnostic notes attached to `notes`

- `"path filtered trigger"` Added on a `never_run` check when every producing workflow fires for `--branch` solely through `paths` or `paths-ignore` triggers.
- `"job carries an if condition"` Added whenever at least one producing job defines a non-empty job-level `if` condition.

## Output Schema (`deadgate-report/1`)

By default (without `--markdown`), Deadgate emits a deterministic `deadgate-report/1` JSON object

- `format` Always `"deadgate-report/1"`.
- `branch` Audited branch name.
- `window` `{"first": "YYYY-MM-DD", "last": "YYYY-MM-DD"}` spanning all timestamps in `--history`, or `null` when history carries no timestamps.
- `checks` Ordered list of `{ "context", "status", "producers", "last_run_date", "notes" }` entries.
- `summary` `{ "required": N, "alive": A, "paper": P }` where `paper = required - alive`.

## Scheduled GitHub Actions Audit Example

```yaml
name: deadgate-audit

on:
  schedule:
    - cron: "0 9 * * 1"
  workflow_dispatch:

permissions:
  contents: read
  actions: read

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Export branch protection and recent job history
        env:
          GH_TOKEN: ${{ secrets.BRANCH_PROTECTION_READ_TOKEN }}
          REPO: ${{ github.repository }}
        run: |
          set -eu
          gh api "repos/$REPO/branches/main/protection" > /tmp/protection.json
          gh api "repos/$REPO/actions/runs?branch=main&per_page=30" \
            --jq '.workflow_runs[] | "\(.id)\t\(.name)"' > /tmp/runs.tsv
          cp /dev/null /tmp/jobs.jsonl
          while IFS="$(printf '\t')" read -r RUN_ID WF_NAME; do
            gh api --paginate "repos/$REPO/actions/runs/$RUN_ID/jobs?per_page=100" > /tmp/run-jobs.json
            jq -c --arg wf "$WF_NAME" '.jobs[] | .workflow_name = (.workflow_name // $wf)' /tmp/run-jobs.json >> /tmp/jobs.jsonl
          done < /tmp/runs.tsv
          rm -f /tmp/runs.tsv /tmp/run-jobs.json
          jq -s '{jobs: .}' /tmp/jobs.jsonl > /tmp/history.json
          rm -f /tmp/jobs.jsonl
      - name: Install deadgate
        run: cargo install --git https://github.com/david-engelmann/deadgate --locked
      - name: Reconcile required checks
        run: |
          deadgate \
            --protection /tmp/protection.json \
            --workflows .github/workflows \
            --history /tmp/history.json > /tmp/deadgate-report.json
          deadgate \
            --protection /tmp/protection.json \
            --workflows .github/workflows \
            --history /tmp/history.json \
            --markdown
          jq -e '.summary.paper == 0' /tmp/deadgate-report.json
```
