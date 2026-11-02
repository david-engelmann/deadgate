"""Reconciliation tests, covering obligations S2-3, S2-4, S2-6, S2-7, and S2-8."""

from deadgate.ingest import load_history, load_protection, load_workflows
from deadgate.reconcile import reconcile

CI = """\
name: ci
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""


def run_reconcile(inputs, contexts, workflow_files, records, branch="main"):
    protection = inputs.protection(contexts)
    folder = inputs.workflows(workflow_files)
    history = inputs.history(records)
    return reconcile(
        load_protection(protection),
        load_workflows(folder),
        load_history(history),
        branch,
    )


def by_context(entries):
    return {entry["context"]: entry for entry in entries}


def test_alive_check_reports_its_last_run(inputs, rec):
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build", completed="2026-10-15T09:04:00Z", started="2026-10-15T09:00:00Z")]
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "alive"
    assert entry["last_run_date"] == "2026-10-15"
    assert len(entry["producers"]) == 1
    assert entry["producers"][0]["job"] == "build"


def test_renamed_job_reads_no_producer_with_a_stale_date(inputs, rec):
    renamed = CI.replace("  build:\n", "  compile:\n")
    entries = run_reconcile(
        inputs,
        ["ci / build"],
        {"ci.yml": renamed},
        [rec("build", started="2026-09-20T09:00:00Z", completed="2026-09-20T09:04:00Z")],
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "no_producer"
    assert entry["producers"] == []
    assert entry["last_run_date"] == "2026-09-20"


def test_produced_check_with_no_records_reads_never_run(inputs):
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, [])
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "never_run"
    assert entry["last_run_date"] is None


def test_only_skipped_records_read_skipped_only(inputs, rec):
    records = [
        rec("build", conclusion="skipped", completed="2026-10-15T09:00:11Z"),
        rec("build", conclusion="skipped", completed="2026-10-14T09:00:11Z"),
    ]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    assert by_context(entries)["ci / build"]["status"] == "skipped_only"


def test_a_run_beside_a_skip_reads_alive(inputs, rec):
    records = [
        rec("build", conclusion="skipped", completed="2026-10-15T09:00:11Z"),
        rec("build", conclusion="failure", completed="2026-10-14T09:04:00Z", started="2026-10-14T09:00:00Z"),
    ]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    assert by_context(entries)["ci / build"]["status"] == "alive"


def test_an_unfinished_record_alone_reads_never_run(inputs, rec):
    records = [rec("build", conclusion=None, status="in_progress", completed=None)]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    assert by_context(entries)["ci / build"]["status"] == "never_run"


def test_schedule_only_workflow_produces_nothing(inputs, rec):
    scheduled = CI.replace(
        "  pull_request:\n    branches: [main]\n  push:\n    branches: [main]\n",
        "  schedule:\n    - cron: '0 0 * * *'\n",
    )
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": scheduled}, [rec("build")]
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "no_producer"
    assert entry["producers"] == []


def test_branches_filter_excludes_the_branch(inputs):
    dev_only = CI.replace("branches: [main]", "branches: [dev]")
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": dev_only}, [])
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_branches_ignore_excludes_the_branch(inputs):
    ignoring = CI.replace(
        "  pull_request:\n    branches: [main]\n",
        "  pull_request:\n    branches-ignore: [main]\n",
    ).replace("  push:\n    branches: [main]\n", "  push:\n    branches-ignore: [main]\n")
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": ignoring}, [])
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_branch_pattern_admits_a_matching_branch(inputs, rec):
    releasing = CI.replace("branches: [main]", "branches: ['release/*']")
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": releasing}, [rec("build")], branch="release/1.0"
    )
    entry = by_context(entries)["ci / build"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"


def test_job_produces_by_display_name_and_by_workflow_form(inputs):
    entries = run_reconcile(inputs, ["build", "ci / build"], {"ci.yml": CI}, [])
    found = by_context(entries)
    assert len(found["build"]["producers"]) == 1
    assert len(found["ci / build"]["producers"]) == 1
    assert found["build"]["producers"][0]["job"] == "build"
    assert found["ci / build"]["producers"][0]["job"] == "build"


def test_matrix_context_is_produced_by_the_matrix_job(inputs, rec):
    matrix = """\
name: ci
on: [pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11", "3.12"]
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs,
        ["ci / test (3.11)"],
        {"ci.yml": matrix},
        [rec("test (3.11)", workflow_name="ci")],
    )
    entry = by_context(entries)["ci / test (3.11)"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"


def test_two_workflows_can_produce_one_check(inputs, rec):
    entries = run_reconcile(
        inputs,
        ["ci / build"],
        {"a.yml": CI, "b.yml": CI},
        [rec("build")],
    )
    entry = by_context(entries)["ci / build"]
    assert [p["file"] for p in entry["producers"]] == ["a.yml", "b.yml"]
    assert entry["status"] == "alive"


def test_last_run_date_is_the_latest_ran_record(inputs, rec):
    records = [
        rec("build", completed="2026-10-10T09:04:00Z", started="2026-10-10T09:00:00Z"),
        rec("build", completed="2026-10-15T09:04:00Z", started="2026-10-15T09:00:00Z"),
        rec("build", conclusion="skipped", completed="2026-10-16T09:00:11Z", started="2026-10-16T09:00:10Z"),
    ]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    assert by_context(entries)["ci / build"]["last_run_date"] == "2026-10-15"


def test_last_run_date_converts_an_offset_timestamp_to_utc(inputs, rec):
    records = [
        rec("build", started="2026-10-15T01:00:00+03:00", completed="2026-10-15T01:30:00+03:00")
    ]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    assert by_context(entries)["ci / build"]["last_run_date"] == "2026-10-14"


def test_skipped_only_check_has_no_last_run_date(inputs, rec):
    records = [rec("build", conclusion="skipped", completed="2026-10-15T09:00:11Z")]
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": CI}, records)
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "skipped_only"
    assert entry["last_run_date"] is None


def test_skipped_job_with_an_if_condition_carries_the_note(inputs, rec):
    guarded = """\
name: ci
on: [pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - run: python -m ruff check .
"""
    entries = run_reconcile(
        inputs,
        ["ci / lint"],
        {"ci.yml": guarded},
        [rec("lint", conclusion="skipped", completed="2026-10-15T09:00:11Z")],
    )
    entry = by_context(entries)["ci / lint"]
    assert entry["status"] == "skipped_only"
    assert "job carries an if condition" in entry["notes"]


def test_path_filtered_producer_with_no_records_reads_never_run_with_the_note(inputs):
    filtered = """\
name: tests
on:
  pull_request:
    branches: [main]
    paths: ["src/**"]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["tests / test"], {"tests.yml": filtered}, [])
    entry = by_context(entries)["tests / test"]
    assert entry["status"] == "never_run"
    assert entry["notes"] == ["path filtered trigger"]


MATRIX_CI = """\
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

PLAIN_CI = """\
name: ci
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""


def test_a_run_from_another_workflow_does_not_certify_the_check(inputs, rec):
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build", workflow_name="nightly")]
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "never_run"
    assert entry["last_run_date"] is None
    assert len(entry["producers"]) == 1


def test_a_workflow_form_record_from_another_workflow_does_not_certify_the_check(inputs, rec):
    entries = run_reconcile(
        inputs,
        ["ci / build"],
        {"ci.yml": CI},
        [rec("ci / build", workflow_name="other")],
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "never_run"
    assert entry["last_run_date"] is None


def test_a_skip_from_another_workflow_does_not_certify_the_check(inputs, rec):
    entries = run_reconcile(
        inputs,
        ["ci / build"],
        {"ci.yml": CI},
        [rec("build", conclusion="skipped", workflow_name="nightly")],
    )
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "never_run"


def test_a_sibling_combination_does_not_certify_the_required_combination(inputs, rec):
    entries = run_reconcile(
        inputs,
        ["ci / test (3.11)"],
        {"ci.yml": MATRIX_CI},
        [rec("test (3.12)", workflow_name="ci")],
    )
    entry = by_context(entries)["ci / test (3.11)"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "never_run"
    assert entry["last_run_date"] is None


def test_a_reordered_combination_does_not_match(inputs, rec):
    two_axes = """\
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11"]
        os: [ubuntu-latest]
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs,
        ["ci / test (3.11, ubuntu-latest)"],
        {"ci.yml": two_axes},
        [rec("test (ubuntu-latest, 3.11)", workflow_name="ci")],
    )
    entry = by_context(entries)["ci / test (3.11, ubuntu-latest)"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "never_run"


def test_a_combination_the_matrix_never_generates_has_no_producer(inputs):
    entries = run_reconcile(inputs, ["ci / test (9.99)"], {"ci.yml": MATRIX_CI}, [])
    entry = by_context(entries)["ci / test (9.99)"]
    assert entry["status"] == "no_producer"
    assert entry["producers"] == []


def test_a_parenthesised_context_against_a_plain_job_has_no_producer(inputs):
    entries = run_reconcile(inputs, ["build (experimental)"], {"ci.yml": PLAIN_CI}, [])
    entry = by_context(entries)["build (experimental)"]
    assert entry["status"] == "no_producer"
    assert entry["producers"] == []


def test_a_parenthesised_record_does_not_certify_the_bare_check(inputs, rec):
    entries = run_reconcile(
        inputs, ["build"], {"ci.yml": PLAIN_CI}, [rec("build (experimental)")]
    )
    entry = by_context(entries)["build"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "never_run"
    assert entry["last_run_date"] is None


def test_an_included_combination_is_produced_and_runs(inputs, rec):
    included = """\
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11"]
        include:
          - python: "3.13"
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs,
        ["ci / test (3.13)"],
        {"ci.yml": included},
        [rec("test (3.13)", workflow_name="ci")],
    )
    entry = by_context(entries)["ci / test (3.13)"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"


def test_an_excluded_combination_has_no_producer(inputs):
    excluded = """\
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python: ["3.11", "3.12"]
        exclude:
          - python: "3.12"
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["ci / test (3.12)"], {"ci.yml": excluded}, [])
    entry = by_context(entries)["ci / test (3.12)"]
    assert entry["status"] == "no_producer"
    assert entry["producers"] == []


def test_a_single_star_pattern_does_not_cross_a_slash(inputs):
    releasing = CI.replace("branches: [main]", "branches: ['release/*']")
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": releasing}, [], branch="release/1.0/hotfix"
    )
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_a_trailing_star_does_not_cross_a_slash(inputs):
    featuring = CI.replace("branches: [main]", "branches: ['feat*']")
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": featuring}, [], branch="feat/x"
    )
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_a_negative_pattern_after_a_positive_excludes_the_branch(inputs):
    ordered = """\
name: ci
on:
  push:
    branches:
      - 'releases/**'
      - '!releases/main'
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": ordered}, [], branch="releases/main"
    )
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_a_positive_pattern_after_a_negative_includes_the_branch_again(inputs, rec):
    ordered = """\
name: ci
on:
  push:
    branches:
      - '!releases/main'
      - 'releases/**'
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": ordered}, [rec("build")], branch="releases/main"
    )
    entry = by_context(entries)["ci / build"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"


def test_a_character_class_pattern_admits_a_matching_branch(inputs, rec):
    classed = CI.replace("branches: [main]", "branches: ['release/v[0-9]']")
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": classed}, [rec("build")], branch="release/v2"
    )
    entry = by_context(entries)["ci / build"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"


def test_a_bare_string_branches_filter_names_one_pattern(inputs):
    bare = """\
name: ci
on:
  push:
    branches: main
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": bare}, [], branch="dev")
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_a_bare_string_branches_ignore_names_one_pattern(inputs):
    bare = """\
name: ci
on:
  push:
    branches-ignore: main
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": bare}, [], branch="main")
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_an_empty_branches_list_admits_no_branch(inputs):
    empty = """\
name: ci
on:
  push:
    branches: []
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": empty}, [], branch="main")
    assert by_context(entries)["ci / build"]["status"] == "no_producer"


def test_the_path_note_does_not_attach_when_an_unfiltered_event_also_fires(inputs):
    dual = """\
name: ci
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
    paths: ["src/**"]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": dual}, [])
    entry = by_context(entries)["ci / build"]
    assert entry["status"] == "never_run"
    assert entry["notes"] == []


def test_the_path_note_does_not_attach_to_an_alive_check(inputs, rec):
    filtered = """\
name: tests
on:
  pull_request:
    branches: [main]
    paths: ["src/**"]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: python -m pytest
"""
    entries = run_reconcile(
        inputs,
        ["tests / test"],
        {"tests.yml": filtered},
        [rec("test", workflow_name="tests")],
    )
    entry = by_context(entries)["tests / test"]
    assert entry["status"] == "alive"
    assert entry["notes"] == []


def test_a_cancelled_record_reads_as_ran(inputs, rec):
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build", conclusion="cancelled")]
    )
    assert by_context(entries)["ci / build"]["status"] == "alive"


def test_a_timed_out_record_reads_as_ran(inputs, rec):
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build", conclusion="timed_out")]
    )
    assert by_context(entries)["ci / build"]["status"] == "alive"


def test_a_neutral_record_reads_as_ran(inputs, rec):
    entries = run_reconcile(
        inputs, ["ci / build"], {"ci.yml": CI}, [rec("build", conclusion="neutral")]
    )
    assert by_context(entries)["ci / build"]["status"] == "alive"


def test_an_alive_job_with_an_if_condition_carries_the_note(inputs, rec):
    guarded = """\
name: ci
on: [pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - run: python -m ruff check .
"""
    entries = run_reconcile(
        inputs, ["ci / lint"], {"ci.yml": guarded}, [rec("lint")]
    )
    entry = by_context(entries)["ci / lint"]
    assert entry["status"] == "alive"
    assert "job carries an if condition" in entry["notes"]


def test_the_single_string_trigger_form_produces_the_check(inputs, rec):
    text = (
        "name: ci\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
        "    steps:\n      - run: true\n"
    )
    entries = run_reconcile(inputs, ["ci / build"], {"ci.yml": text}, [rec("build")])
    entry = by_context(entries)["ci / build"]
    assert len(entry["producers"]) == 1
    assert entry["status"] == "alive"
