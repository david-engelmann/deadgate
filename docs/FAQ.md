# Frequently Asked Questions

## Why does Deadgate take local JSON exports instead of calling the GitHub API itself?

Security and compliance teams frequently audit branch-protection posture across restricted orgs where handing third-party binaries a `repo` or `administration:read` token is unacceptable. Because Deadgate makes zero network calls and spawns zero subprocesses, an operator exports two JSON files with standard `gh api`, archives them as audit evidence, and runs `deadgate` completely offline.

## Why does Deadgate exit `0` when paper checks are found?

Finding paper checks is a successful audit report, not a tool usage error (exit `2` is reserved for malformed inputs, duplicate JSON keys, or unreadable files). In CI gates, assert `jq -e '.summary.paper == 0' report.json` to fail the workflow whenever `summary.paper > 0`.

## How does `no_producer` still know `last_run_date` (`2026-09-20`) after a job rename?

When zero current workflow jobs can produce a required context for `--branch`, Deadgate scans the execution history window for historical runs matching that context name so operators can see the exact date the check stopped running after a rename.

## Does Deadgate support YAML anchors, merge keys (`<<`), and multi-line block scalars in workflows?

Yes. Deadgate's pure-Rust YAML 1.1 reader is proven against recorded PyYAML 6.0.1 conformance batteries across literal/folded block scalars, anchors, aliases, merge keys, tags, and explicit `?` keys under a `4000` expansion bound.
