"""Command line tests, covering obligation S2-10."""

import json

from deadgate.cli import main

CI = """\
name: ci
on: [pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""


def make_inputs(inputs, rec):
    protection = inputs.protection(["ci / build"])
    folder = inputs.workflows({"ci.yml": CI})
    history = inputs.history([rec("build")])
    return protection, folder, history


def test_cli_prints_the_json_report(inputs, rec, capsys):
    protection, folder, history = make_inputs(inputs, rec)
    code = main(["--protection", protection, "--workflows", folder, "--history", history])
    assert code == 0
    parsed = json.loads(capsys.readouterr().out)
    assert parsed["format"] == "deadgate-report/1"
    assert parsed["checks"][0]["status"] == "alive"


def test_cli_prints_the_markdown_report(inputs, rec, capsys):
    protection, folder, history = make_inputs(inputs, rec)
    code = main(
        ["--protection", protection, "--workflows", folder, "--history", history, "--markdown"]
    )
    assert code == 0
    out = capsys.readouterr().out
    assert out.startswith("# deadgate report")
    assert "| ci / build | ci / build | alive | 2026-10-14 |" in out


def test_cli_missing_protection_file_is_a_named_error(inputs, rec, capsys, tmp_path):
    missing = str(tmp_path / "nope.json")
    folder = inputs.workflows({"ci.yml": CI})
    history = inputs.history([rec("build")])
    code = main(["--protection", missing, "--workflows", folder, "--history", history])
    assert code != 0
    err = capsys.readouterr().err
    assert missing in err
    assert "Traceback" not in err


def test_cli_refused_protection_is_a_named_error(inputs, rec, capsys):
    protection = inputs.protection(None, style={})
    folder = inputs.workflows({"ci.yml": CI})
    history = inputs.history([rec("build")])
    code = main(["--protection", protection, "--workflows", folder, "--history", history])
    assert code != 0
    err = capsys.readouterr().err
    assert "no required status checks" in err
    assert "Traceback" not in err
