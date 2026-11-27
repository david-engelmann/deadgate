# Security Policy

## Supported Versions

Security fixes land on the latest `0.24.x` and subsequent `0.x` release line on `main`. Earlier tags do not receive backported patches.

## Security Model

Deadgate makes zero network calls, spawns zero external processes, and reads only local export files handed to it on the command line (`--protection`, `--workflows`, `--history`). Its JSON and YAML readers enforce explicit nesting and alias/merge expansion bounds (`4000`) and strict duplicate-key refusal on JSON inputs so untrusted or malformed exports fail closed without unbounded memory expansion. Never commit real branch-protection payloads, workflow files, or run histories from private or production repositories into test fixtures.

## Reporting a Vulnerability

Do not open a public issue for a suspected security vulnerability. Report vulnerabilities privately through [GitHub Security Advisories](https://github.com/david-engelmann/deadgate/security/advisories/new) with synthetic inputs only, naming the affected version and the relevant section of `CONTRACT.md`. Every report receives an initial review within seven days, and confirmed fixes ship with a regression pin in the test suite before public disclosure.
