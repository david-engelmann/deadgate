"""The report, under obligation S2-9."""

from __future__ import annotations

import json

FORMAT = "deadgate-report/1"

STATUS_WORDS = {
    "alive": "alive",
    "no_producer": "no producer",
    "never_run": "never ran",
    "skipped_only": "skipped only",
}


def build_report(contexts, workflows, records, branch, window):
    from .reconcile import reconcile

    checks = reconcile(contexts, workflows, records, branch)
    alive = sum(1 for check in checks if check["status"] == "alive")
    return {
        "format": FORMAT,
        "branch": branch,
        "window": window,
        "checks": checks,
        "summary": {
            "required": len(checks),
            "alive": alive,
            "paper": len(checks) - alive,
        },
    }


def render_json(report):
    return json.dumps(report, indent=2) + "\n"


def render_markdown(report):
    lines = ["# deadgate report", ""]
    window = report["window"]
    if window is None:
        lines.append(f"Branch {report['branch']}. History window none.")
    else:
        lines.append(
            f"Branch {report['branch']}. "
            f"History window {window['first']} to {window['last']}."
        )
    lines.append("")
    lines.append("| Required check | Producing job | Status | Last ran |")
    lines.append("| --- | --- | --- | --- |")
    for check in report["checks"]:
        if check["producers"]:
            producing = ", ".join(
                f"{p['workflow']} / {p['job_name']}" for p in check["producers"]
            )
        else:
            producing = "none"
        last_ran = check["last_run_date"] or "never"
        status = STATUS_WORDS[check["status"]]
        if check["notes"]:
            status = status + " (" + ", ".join(check["notes"]) + ")"
        lines.append(f"| {check['context']} | {producing} | {status} | {last_ran} |")
    return "\n".join(lines) + "\n"
