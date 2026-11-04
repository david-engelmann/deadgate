"""Round 2 pins for the duplicate JSON key refusal, obligation R2-1."""

import pytest

from deadgate.cli import main
from deadgate.ingest import DeadgateError, load_history, load_protection, load_workflows


def _write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return str(path)


def test_duplicate_keys_in_the_protection_export_are_refused(tmp_path):
    path = _write(
        tmp_path,
        "protection.json",
        '{"required_status_checks": {"contexts": ["ci / build"]}, '
        '"required_status_checks": {"contexts": ["ci / lint"]}}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_protection(path)
    message = str(caught.value)
    assert "protection export" in message
    assert "required_status_checks" in message
    assert "more than once" in message


def test_duplicate_keys_in_a_checks_entry_are_refused(tmp_path):
    path = _write(
        tmp_path,
        "protection.json",
        '{"required_status_checks": {"checks": ['
        '{"context": "ci / ghost", "context": "ci / build", "app_id": 15368}'
        "]}}",
    )
    with pytest.raises(DeadgateError) as caught:
        load_protection(path)
    assert "context" in str(caught.value)


def test_duplicate_keys_in_the_history_export_are_refused(tmp_path, capsys):
    path = _write(
        tmp_path,
        "history.json",
        '{"jobs": [{"name": "build", "workflow_name": "ci", '
        '"status": "completed", "conclusion": "failure", '
        '"conclusion": "success", '
        '"started_at": "2026-10-14T09:00:00Z", '
        '"completed_at": "2026-10-14T09:04:00Z"}]}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "conclusion" in str(caught.value)
    protection = _write(
        tmp_path,
        "protection.json",
        '{"required_status_checks": {"contexts": ["ci / build"]}}',
    )
    folder = tmp_path / "workflows"
    folder.mkdir()
    (folder / "ci.yml").write_text(
        "name: ci\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
        encoding="utf-8",
    )
    code = main(
        [
            "--protection", protection,
            "--workflows", str(folder),
            "--history", path,
        ]
    )
    assert code == 2
    err = capsys.readouterr().err
    assert "conclusion" in err
    assert "Traceback" not in err


def test_duplicate_keys_refuse_at_any_depth_and_name_the_first_repeat(tmp_path):
    path = _write(
        tmp_path,
        "history.json",
        '{"jobs": [{"name": "build", "name": "lint", "workflow_name": "ci", '
        '"status": "completed", "conclusion": "success", '
        '"started_at": "2026-10-14T09:00:00Z", '
        '"completed_at": "2026-10-14T09:04:00Z"}], "jobs": []}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "name" in str(caught.value)
    path = _write(
        tmp_path,
        "history.json",
        '{"jobs": [{"name": "build", "workflow_name": "ci", '
        '"status": "completed", "conclusion": "success", '
        '"started_at": "2026-10-14T09:00:00Z", '
        '"completed_at": "2026-10-14T09:04:00Z", '
        '"extra": {"x": 1, "y": 2, "x": 3, "y": 4}}]}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "key x more than once" in str(caught.value)
    path = _write(
        tmp_path,
        "history.json",
        '{"jobs": [{"name": "build", "workflow_name": "ci", '
        '"status": "completed", "conclusion": "success", '
        '"started_at": "2026-10-14T09:00:00Z", '
        '"completed_at": "2026-10-14T09:04:00Z", '
        '"extra": [{"k": 1, "k": 2}]}]}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "key k more than once" in str(caught.value)


def test_an_escaped_twin_key_is_a_repeat(tmp_path):
    path = _write(
        tmp_path,
        "history.json",
        '{"jobs": [{"name": "build", "workflow_name": "ci", '
        '"status": "completed", "conclusion": "failure", '
        '"conclus\\u0069on": "success", '
        '"started_at": "2026-10-14T09:00:00Z", '
        '"completed_at": "2026-10-14T09:04:00Z"}]}',
    )
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "conclusion" in str(caught.value)


def test_the_same_key_in_different_objects_is_accepted(tmp_path, rec, inputs):
    path = inputs.history([rec("build"), rec("lint")])
    records = load_history(path)
    assert [record["name"] for record in records] == ["build", "lint"]
    path = inputs.protection(["ci / build", "ci / lint"])
    assert load_protection(path) == ["ci / build", "ci / lint"]


def test_a_repeated_yaml_key_keeps_its_standing_reading(inputs):
    folder = inputs.workflows(
        {
            "ci.yml": (
                "name: first\nname: second\non: [push]\njobs:\n"
                "  build:\n    runs-on: ubuntu-latest\n"
            )
        }
    )
    workflows = load_workflows(folder)
    assert workflows[0]["workflow"] == "second"
