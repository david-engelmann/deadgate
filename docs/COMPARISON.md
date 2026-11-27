# How Deadgate Compares

Silent branch-protection rot is one of the most dangerous failure modes in modern CI. Rename a workflow job from `package` to `build-package` (or add a job-level `if` condition that always evaluates false on pull requests), and GitHub's branch protection either waits forever on path-filtered checks or treats skipped jobs as green passing checks while enforcing nothing.

## Ecosystem Comparison

| Capability | GitHub Branch Protection UI | Workflow linters (`actionlint`, `zizmor`) | Policy scanners (`scorecard`, `repostatus`) | `deadgate` |
| --- | --- | --- | --- | --- |
| Detects renamed required checks (`no_producer`) with last stale run date | Shows string name only, no workflow cross-check | Checks single YAML syntax, unaware of branch protection | Checks whether branch protection is enabled, not check liveness | **Yes (`no_producer` + stale date)** |
| Distinguishes `skipped_only` (`if` bypass) from `never_run` (`paths` filter) | Marks skipped jobs green | No execution history awareness | No | **Yes (two distinct paper verdicts + notes)** |
| Expands `strategy.matrix` (`include` / `exclude`) and ordered branch filters | At runtime only | Partial static check | No | **Yes (exact context matching)** |
| Runs 100% air-gapped with zero API tokens or network calls | Cloud UI only | Yes | Requires GitHub token | **Yes (pure-Rust local file ingest)** |

## Sibling Boundaries in Gate Truth

- **Deadgate** asks *whether the gate ran* (`alive` vs `no_producer`, `never_run`, `skipped_only`).
- **Finaltree** asks *what tree it ran on*.
- **Blindgate** asks *what files it actually opened* while running.
