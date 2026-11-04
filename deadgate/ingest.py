"""Ingestion of the three local inputs, under obligations S2-1, S2-2, and
S2-5, as amended by the Round 1 review fixes FIX-2, FIX-6, and FIX-7 and
the Round 2 amendment R2-1."""

from __future__ import annotations

import itertools
import json
from datetime import datetime, timezone
from pathlib import Path

import yaml


class DeadgateError(Exception):
    """A reported error naming the input that caused it."""


def _object_without_repeated_keys(pairs, label, path):
    """Build one JSON object, refusing a key that stands twice, under R2-1."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise DeadgateError(
                f"{label} at {path} carries the key {key} more than once"
            )
        result[key] = value
    return result


def _read_json(path, label):
    try:
        text = Path(path).read_text(encoding="utf-8")
    except OSError:
        raise DeadgateError(f"{label} cannot be read at {path}") from None
    try:
        return json.loads(
            text,
            object_pairs_hook=lambda pairs: _object_without_repeated_keys(
                pairs, label, path
            ),
        )
    except json.JSONDecodeError:
        raise DeadgateError(f"{label} at {path} is not valid JSON") from None


def load_protection(path):
    """The required status check contexts of one branch, in file order."""
    data = _read_json(path, "protection export")
    if not isinstance(data, dict) or not isinstance(
        data.get("required_status_checks"), dict
    ):
        raise DeadgateError(
            f"protection export at {path} carries no required status checks"
        )
    required = data["required_status_checks"]
    checks = required.get("checks")
    if checks is not None and not isinstance(checks, list):
        raise DeadgateError(
            f"protection export at {path} carries a checks value that "
            "is not a list"
        )
    contexts = []
    if isinstance(checks, list) and checks:
        for entry in checks:
            context = entry.get("context") if isinstance(entry, dict) else None
            if not isinstance(context, str) or not context:
                raise DeadgateError(
                    f"protection export at {path} carries a context that "
                    "is not a non empty string"
                )
            contexts.append(context)
    else:
        legacy = required.get("contexts")
        if isinstance(legacy, list):
            for context in legacy:
                if not isinstance(context, str) or not context:
                    raise DeadgateError(
                        f"protection export at {path} carries a context that "
                        "is not a non empty string"
                    )
                contexts.append(context)
    if not contexts:
        raise DeadgateError(
            f"protection export at {path} carries no required status checks"
        )
    seen = []
    for context in contexts:
        if context not in seen:
            seen.append(context)
    return seen


def _trigger_block(data):
    # The YAML parser hands the unquoted on key back as the boolean True.
    if "on" in data:
        return data["on"]
    return data.get(True)


def _scalar_text(value):
    """A matrix value in its plain scalar form, for comparing and rendering."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    return str(value)


def _matrix_combinations(spec):
    """The combination strings the job strategy matrix generates, under
    FIX-2, or None when the job carries no matrix."""
    strategy = spec.get("strategy")
    if not isinstance(strategy, dict):
        return None
    matrix = strategy.get("matrix")
    if not isinstance(matrix, dict):
        return None
    axes = [key for key in matrix if key not in ("include", "exclude")]
    includes = matrix.get("include")
    if not isinstance(includes, list):
        includes = []
    includes = [entry for entry in includes if isinstance(entry, dict)]
    excludes = matrix.get("exclude")
    if not isinstance(excludes, list):
        excludes = []
    excludes = [entry for entry in excludes if isinstance(entry, dict)]

    def render(combination):
        keys = [axis for axis in axes if axis in combination]
        keys += [key for key in combination if key not in axes]
        return ", ".join(_scalar_text(combination[key]) for key in keys)

    if not axes:
        combinations = [dict(entry) for entry in includes]
    else:
        originals = []
        for values in itertools.product(
            *[
                matrix[axis] if isinstance(matrix[axis], list) else [matrix[axis]]
                for axis in axes
            ]
        ):
            originals.append(dict(zip(axes, values)))
        combinations = [dict(original) for original in originals]
        for entry in includes:
            merged = False
            for position, original in enumerate(originals):
                overwrites = any(
                    key in original
                    and _scalar_text(original[key]) != _scalar_text(value)
                    for key, value in entry.items()
                )
                if not overwrites:
                    combinations[position].update(entry)
                    merged = True
            if not merged:
                combinations.append(dict(entry))
    kept = []
    for combination in combinations:
        excluded = any(
            all(
                key in combination
                and _scalar_text(combination[key]) == _scalar_text(value)
                for key, value in entry.items()
            )
            for entry in excludes
        )
        if not excluded:
            kept.append(render(combination))
    return kept


def load_workflows(folder):
    """Every workflow definition in a folder, in filename order."""
    folder = Path(folder)
    if not folder.is_dir():
        raise DeadgateError(f"workflows folder cannot be read at {folder}")
    files = sorted(
        p for p in folder.iterdir() if p.suffix in (".yml", ".yaml") and p.is_file()
    )
    workflows = []
    for path in files:
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            raise DeadgateError(f"workflow file cannot be read at {path}") from None
        try:
            data = yaml.safe_load(text)
        except yaml.YAMLError:
            raise DeadgateError(f"workflow file at {path} does not parse") from None
        if not isinstance(data, dict):
            raise DeadgateError(f"workflow file at {path} is not a mapping")
        jobs = data.get("jobs")
        if not isinstance(jobs, dict) or not jobs:
            raise DeadgateError(f"workflow file at {path} carries no jobs")
        name = data.get("name")
        if not isinstance(name, str) or not name:
            name = path.stem
        job_map = {}
        for job_id, spec in jobs.items():
            spec = spec if isinstance(spec, dict) else {}
            display = spec.get("name")
            if not isinstance(display, str) or not display:
                display = str(job_id)
            job_map[str(job_id)] = {
                "job": str(job_id),
                "job_name": display,
                "has_if": isinstance(spec.get("if"), str) and bool(spec.get("if")),
                "combinations": _matrix_combinations(spec),
            }
        workflows.append(
            {
                "workflow": name,
                "file": path.name,
                "triggers": _trigger_block(data),
                "jobs": job_map,
            }
        )
    return workflows


def parse_timestamp(value):
    """An ISO 8601 timestamp as an aware datetime in UTC, or None."""
    if value is None:
        return None
    if not isinstance(value, str):
        raise ValueError("timestamp is not a string")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def load_history(path):
    """The job records of an execution history export, in file order."""
    data = _read_json(path, "history export")
    if not isinstance(data, dict) or not isinstance(data.get("jobs"), list):
        raise DeadgateError(f"history export at {path} carries no jobs list")
    records = []
    for position, entry in enumerate(data["jobs"]):
        if not isinstance(entry, dict):
            raise DeadgateError(f"history record {position} is not a mapping")
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            raise DeadgateError(
                f"history record {position} carries a name that is not "
                "a non empty string"
            )
        try:
            started = parse_timestamp(entry.get("started_at"))
            completed = parse_timestamp(entry.get("completed_at"))
        except ValueError:
            raise DeadgateError(
                f"history record {position} carries a timestamp that does not parse"
            ) from None
        if entry.get("status") == "completed" and started is None and completed is None:
            raise DeadgateError(
                f"history record {position} is completed and carries "
                "neither a start nor a completion timestamp"
            )
        records.append(
            {
                "name": name,
                "workflow_name": entry.get("workflow_name"),
                "status": entry.get("status"),
                "conclusion": entry.get("conclusion"),
                "started_at": started,
                "completed_at": completed,
            }
        )
    return records


def history_window(records):
    """The first and last dates any record touches, or None when empty."""
    stamps = []
    for record in records:
        for key in ("started_at", "completed_at"):
            if record[key] is not None:
                stamps.append(record[key])
    if not stamps:
        return None
    return {
        "first": min(stamps).date().isoformat(),
        "last": max(stamps).date().isoformat(),
    }
