"""The command line entry point, under obligation S2-10."""

from __future__ import annotations

import argparse
import sys

from .ingest import DeadgateError, history_window, load_history, load_protection, load_workflows
from .report import build_report, render_json, render_markdown


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="deadgate",
        description=(
            "Reconcile required status checks against the jobs CI "
            "actually executes and the history it left behind."
        ),
    )
    parser.add_argument("--protection", required=True, help="branch protection export, JSON")
    parser.add_argument("--workflows", required=True, help="folder of workflow definitions")
    parser.add_argument("--history", required=True, help="execution history export, JSON")
    parser.add_argument("--branch", default="main", help="branch the protection export covers")
    parser.add_argument(
        "--markdown",
        action="store_true",
        help="print the markdown rendering instead of the JSON rendering",
    )
    args = parser.parse_args(argv)
    try:
        contexts = load_protection(args.protection)
        workflows = load_workflows(args.workflows)
        records = load_history(args.history)
    except DeadgateError as error:
        print(f"deadgate: {error}", file=sys.stderr)
        return 2
    report = build_report(
        contexts, workflows, records, args.branch, history_window(records)
    )
    sys.stdout.write(render_markdown(report) if args.markdown else render_json(report))
    return 0
