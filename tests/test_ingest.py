"""Ingestion tests, covering obligations S2-1, S2-2, and S2-5."""

import pytest

from deadgate.ingest import DeadgateError, load_history, load_protection, load_workflows

CI_WORKFLOW = """\
name: ci
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
jobs:
  build:
    name: Build job
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
  lint:
    runs-on: ubuntu-latest
    steps:
      - run: python -m ruff check .
"""


def test_protection_reads_the_checks_list_in_order(inputs):
    payload = {
        "required_status_checks": {
            "contexts": ["legacy / ignored"],
            "checks": [
                {"context": "ci / lint", "app_id": 15368},
                {"context": "ci / build", "app_id": 15368},
            ],
        }
    }
    path = inputs.protection(None, style=payload)
    assert load_protection(path) == ["ci / lint", "ci / build"]


def test_protection_falls_back_to_the_legacy_contexts(inputs):
    path = inputs.protection(["ci / build", "ci / lint"], style="legacy")
    assert load_protection(path) == ["ci / build", "ci / lint"]
    payload = {"required_status_checks": {"checks": [], "contexts": ["ci / build"]}}
    path = inputs.protection(None, style=payload)
    assert load_protection(path) == ["ci / build"]


def test_protection_collapses_a_duplicated_context(inputs):
    path = inputs.protection(["ci / build", "ci / lint", "ci / build"])
    assert load_protection(path) == ["ci / build", "ci / lint"]


def test_protection_without_required_checks_is_an_error(inputs):
    path = inputs.protection(None, style={})
    with pytest.raises(DeadgateError):
        load_protection(path)
    payload = {"required_status_checks": {"checks": [], "contexts": []}}
    path = inputs.protection(None, style=payload)
    with pytest.raises(DeadgateError):
        load_protection(path)


def test_protection_with_a_blank_context_is_an_error(inputs):
    path = inputs.protection(["ci / build", ""])
    with pytest.raises(DeadgateError):
        load_protection(path)
    path = inputs.protection([""], style="legacy")
    with pytest.raises(DeadgateError):
        load_protection(path)


def test_workflow_reads_name_jobs_and_display_names(inputs):
    folder = inputs.workflows({"ci.yml": CI_WORKFLOW})
    workflows = load_workflows(folder)
    assert len(workflows) == 1
    workflow = workflows[0]
    assert workflow["workflow"] == "ci"
    assert workflow["jobs"]["build"]["job_name"] == "Build job"
    assert workflow["jobs"]["lint"]["job_name"] == "lint"
    assert set(workflow["triggers"]) == {"pull_request", "push"}
    assert workflow["triggers"]["push"]["branches"] == ["main"]
    assert workflow["triggers"]["pull_request"]["branches"] == ["main"]


def test_protection_with_a_non_list_checks_value_is_an_error(inputs):
    payload = {
        "required_status_checks": {
            "checks": {"context": "ci / build"},
            "contexts": ["ci / build"],
        }
    }
    path = inputs.protection(None, style=payload)
    with pytest.raises(DeadgateError) as caught:
        load_protection(path)
    assert "not a list" in str(caught.value)


def test_workflow_that_does_not_parse_is_an_error_naming_the_file(inputs):
    folder = inputs.workflows({"bad.yml": "name: bad\non: [push\n"})
    with pytest.raises(DeadgateError) as caught:
        load_workflows(folder)
    assert "bad.yml" in str(caught.value)


MATRIX_WORKFLOW = """\
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11", "3.12"]
    steps:
      - run: python -m pytest
"""


def test_workflow_reads_matrix_combinations(inputs):
    folder = inputs.workflows({"ci.yml": MATRIX_WORKFLOW})
    workflows = load_workflows(folder)
    assert workflows[0]["jobs"]["test"]["combinations"] == ["3.11", "3.12"]


def test_workflow_generates_the_documented_matrix_combinations(inputs):
    documented = """\
name: ci
on: [push]
jobs:
  check:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        fruit: [apple, pear]
        animal: [cat, dog]
        include:
          - color: green
          - color: pink
            animal: cat
          - fruit: apple
            shape: circle
          - fruit: banana
          - fruit: banana
            animal: cat
    steps:
      - run: python -m pytest
"""
    folder = inputs.workflows({"ci.yml": documented})
    workflows = load_workflows(folder)
    assert workflows[0]["jobs"]["check"]["combinations"] == [
        "apple, cat, pink, circle",
        "apple, dog, green, circle",
        "pear, cat, pink",
        "pear, dog, green",
        "banana",
        "banana, cat",
    ]


def test_workflow_exclude_removes_the_combinations_it_matches(inputs):
    excluded = """\
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11", "3.12"]
        os: [ubuntu-latest, windows-latest]
        exclude:
          - python: "3.12"
            os: windows-latest
    steps:
      - run: python -m pytest
"""
    folder = inputs.workflows({"ci.yml": excluded})
    workflows = load_workflows(folder)
    assert workflows[0]["jobs"]["test"]["combinations"] == [
        "3.11, ubuntu-latest",
        "3.11, windows-latest",
        "3.12, ubuntu-latest",
    ]


def test_workflow_name_falls_back_to_the_filename_stem(inputs):
    text = "on: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n"
    folder = inputs.workflows({"nightly.yml": text})
    workflows = load_workflows(folder)
    assert workflows[0]["workflow"] == "nightly"


def test_workflow_without_jobs_is_an_error_naming_the_file(inputs):
    folder = inputs.workflows({"broken.yml": "name: broken\non: [push]\n"})
    with pytest.raises(DeadgateError) as caught:
        load_workflows(folder)
    assert "broken.yml" in str(caught.value)


def test_workflow_yaml_suffix_is_read(inputs):
    folder = inputs.workflows({"a.yml": CI_WORKFLOW, "b.yaml": CI_WORKFLOW})
    assert len(load_workflows(folder)) == 2


def test_history_reads_the_job_records(inputs, rec):
    records = [
        rec("build", conclusion="success"),
        rec("lint", conclusion=None, status="in_progress", completed=None),
    ]
    path = inputs.history(records)
    loaded = load_history(path)
    assert [r["name"] for r in loaded] == ["build", "lint"]
    assert loaded[0]["conclusion"] == "success"
    assert loaded[0]["completed_at"].isoformat() == "2026-10-14T09:04:00+00:00"
    assert loaded[1]["conclusion"] is None
    assert loaded[1]["completed_at"] is None


def test_history_with_a_bad_timestamp_is_an_error_naming_the_position(inputs, rec):
    records = [rec("build"), rec("lint", started="not a timestamp")]
    path = inputs.history(records)
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "record 1" in str(caught.value)


def test_history_with_a_blank_name_is_an_error(inputs, rec):
    path = inputs.history([rec("")])
    with pytest.raises(DeadgateError):
        load_history(path)


def test_history_with_a_completed_record_carrying_no_timestamp_is_an_error(inputs, rec):
    records = [rec("build"), rec("lint", started=None, completed=None)]
    path = inputs.history(records)
    with pytest.raises(DeadgateError) as caught:
        load_history(path)
    assert "record 1" in str(caught.value)
