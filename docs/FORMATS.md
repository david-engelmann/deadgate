# Deadgate Wire Formats (`deadgate-report/1`)

Deadgate reads three local input formats and emits either the pinned `deadgate-report/1` JSON schema (default) or a deterministic Markdown table (`--markdown`).

## Input Schemas

1. **Branch Protection (`--protection`)** JSON object carrying `required_status_checks.checks` (array of `{"context": "<non-empty string>"}` objects) or `required_status_checks.contexts` (array of non-empty strings). Duplicate contexts are deduplicated in first-seen order.
2. **Workflow Directory (`--workflows`)** Directory containing GitHub Actions `.yml` / `.yaml` files.
3. **Execution History (`--history`)** JSON object carrying `"jobs"` as an array of job objects (`name`, `workflow_name`, `status`, `conclusion`, `started_at`, `completed_at`). Completed records require at least one ISO 8601 timestamp.

## Output JSON Schema (`deadgate-report/1`)

Top-level keys are emitted in fixed insertion order (`format`, `branch`, `window`, `checks`, `summary`)

- `format` Always `"deadgate-report/1"`.
- `branch` Target branch name (default `"main"`).
- `window` Object `{"first": "<YYYY-MM-DD>", "last": "<YYYY-MM-DD>"}` spanning the earliest and latest valid ISO 8601 dates across `started_at` and `completed_at` on every history record regardless of job status, or `null` when history carries zero valid timestamps.
- `checks` Array of reconciled required check objects (`context`, `status`, `producers`, `last_run_date`, `notes`), where `producers` is an array of `{"workflow", "file", "job", "job_name"}` objects naming every workflow job capable of producing `context` on `branch`.
- `summary` Object `{"required", "alive", "paper"}` counting total deduplicated required checks, `alive` checks, and paper checks (`skipped_only` + `never_run` + `no_producer`).

```json
{
  "format": "deadgate-report/1",
  "branch": "main",
  "window": {
    "first": "2026-09-20",
    "last": "2026-10-15"
  },
  "checks": [
    {
      "context": "ci / build",
      "status": "alive",
      "producers": [
        {
          "workflow": "ci",
          "file": "ci.yml",
          "job": "build",
          "job_name": "build"
        }
      ],
      "last_run_date": "2026-10-15",
      "notes": []
    },
    {
      "context": "ci / lint",
      "status": "skipped_only",
      "producers": [
        {
          "workflow": "ci",
          "file": "ci.yml",
          "job": "lint",
          "job_name": "lint"
        }
      ],
      "last_run_date": null,
      "notes": [
        "job carries an if condition"
      ]
    },
    {
      "context": "ci / package",
      "status": "no_producer",
      "producers": [],
      "last_run_date": "2026-09-20",
      "notes": []
    },
    {
      "context": "tests / test",
      "status": "never_run",
      "producers": [
        {
          "workflow": "tests",
          "file": "tests.yml",
          "job": "test",
          "job_name": "test"
        }
      ],
      "last_run_date": null,
      "notes": [
        "path filtered trigger"
      ]
    }
  ],
  "summary": {
    "required": 4,
    "alive": 1,
    "paper": 3
  }
}
```
