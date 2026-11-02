"""Report tests, covering obligation S2-9."""

import json
from pathlib import Path

from deadgate.cli import main
from deadgate.ingest import history_window, load_history, load_protection, load_workflows
from deadgate.report import build_report, render_json, render_markdown

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"

CI = """\
name: ci
on: [pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""


def build_from(inputs, contexts, workflow_files, records, branch="main"):
    protection = inputs.protection(contexts)
    folder = inputs.workflows(workflow_files)
    history = inputs.history(records)
    loaded_records = load_history(history)
    return build_report(
        load_protection(protection),
        load_workflows(folder),
        loaded_records,
        branch,
        history_window(loaded_records),
    )


def test_json_report_carries_the_whole_reconciliation(inputs, rec):
    report = build_from(
        inputs,
        ["ci / build", "ci / lint"],
        {"ci.yml": CI},
        [rec("build", started="2026-10-14T09:00:00Z", completed="2026-10-15T09:04:00Z")],
    )
    assert report["format"] == "deadgate-report/1"
    assert report["branch"] == "main"
    assert report["window"] == {"first": "2026-10-14", "last": "2026-10-15"}
    assert [c["context"] for c in report["checks"]] == ["ci / build", "ci / lint"]
    assert report["checks"][0]["status"] == "alive"
    assert report["checks"][0]["last_run_date"] == "2026-10-15"
    assert report["checks"][1]["status"] == "no_producer"
    assert report["summary"] == {"required": 2, "alive": 1, "paper": 1}
    parsed = json.loads(render_json(report))
    assert parsed == report


def test_markdown_report_carries_the_same_facts(inputs, rec):
    report = build_from(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build")]
    )
    markdown = render_markdown(report)
    assert "| Required check | Producing job | Status | Last ran |" in markdown
    assert "| ci / build | ci / build | alive | 2026-10-14 |" in markdown
    assert "History window 2026-10-14 to 2026-10-14." in markdown


def test_report_is_byte_identical_across_two_runs(inputs, rec):
    first = build_from(inputs, ["ci / build"], {"ci.yml": CI}, [rec("build")])
    second = build_from(inputs, ["ci / build"], {"ci.yml": CI}, [rec("build")])
    assert render_json(first) == render_json(second)
    assert render_markdown(first) == render_markdown(second)


def test_example_reports_regenerate_byte_identical(capsys):
    code = main(
        [
            "--protection", str(EXAMPLES / "protection.json"),
            "--workflows", str(EXAMPLES / "workflows"),
            "--history", str(EXAMPLES / "history.json"),
        ]
    )
    assert code == 0
    assert capsys.readouterr().out == (EXAMPLES / "report.json").read_text(encoding="utf-8")
    code = main(
        [
            "--protection", str(EXAMPLES / "protection.json"),
            "--workflows", str(EXAMPLES / "workflows"),
            "--history", str(EXAMPLES / "history.json"),
            "--markdown",
        ]
    )
    assert code == 0
    assert capsys.readouterr().out == (EXAMPLES / "report.md").read_text(encoding="utf-8")
