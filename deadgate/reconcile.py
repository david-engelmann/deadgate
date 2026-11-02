"""The reconciliation, under obligations S2-3, S2-4, S2-6, S2-7, and S2-8,
as amended by the Round 1 review fixes FIX-1 to FIX-5."""

from __future__ import annotations

import re

RAN_CONCLUSIONS = {"success", "failure", "cancelled", "timed_out", "neutral"}
BRANCH_EVENTS = ("push", "pull_request")

NOTE_PATH_FILTERED = "path filtered trigger"
NOTE_IF_CONDITION = "job carries an if condition"


def _event_configs(triggers):
    """Trigger events mapped to their config dicts, from any trigger form."""
    if isinstance(triggers, str):
        return {triggers: {}}
    if isinstance(triggers, list):
        return {str(event): {} for event in triggers}
    if isinstance(triggers, dict):
        configs = {}
        for event, config in triggers.items():
            configs[str(event)] = config if isinstance(config, dict) else {}
        return configs
    return {}


def _glob_body(pattern):
    """One GitHub filter pattern as a regular expression body, under FIX-3."""
    atoms = []
    quantified = False

    def emit(atom):
        nonlocal quantified
        atoms.append(atom)
        quantified = False

    index = 0
    while index < len(pattern):
        char = pattern[index]
        if char == "\\" and index + 1 < len(pattern):
            emit(re.escape(pattern[index + 1]))
            index += 2
        elif char == "*":
            if index + 1 < len(pattern) and pattern[index + 1] == "*":
                emit(".*")
                index += 2
            else:
                emit("[^/]*")
                index += 1
        elif char in ("?", "+"):
            if atoms and not quantified:
                atoms[-1] = atoms[-1] + char
                quantified = True
            else:
                emit(re.escape(char))
            index += 1
        elif char == "[":
            closing = pattern.find("]", index + 1)
            if closing > index + 1:
                inner = pattern[index + 1 : closing].replace("\\", "\\\\")
                if inner.startswith("^"):
                    inner = "\\" + inner
                emit("[" + inner + "]")
                index = closing + 1
            else:
                emit(re.escape(char))
                index += 1
        else:
            emit(re.escape(char))
            index += 1
    return "".join(atoms)


def _pattern_matches(pattern, name):
    return re.fullmatch(_glob_body(pattern), name) is not None


def _pattern_list_claims(patterns, name):
    """Whether a pattern list claims a name, its patterns applied in the
    order the file writes them so the last matching pattern decides."""
    claimed = False
    for pattern in patterns:
        if pattern.startswith("!"):
            if _pattern_matches(pattern[1:], name):
                claimed = False
        elif _pattern_matches(pattern, name):
            claimed = True
    return claimed


def _filter_patterns(config, key):
    """The patterns one branch filter key carries, or None when the key
    stands absent, under FIX-4."""
    if key not in config:
        return None
    value = config[key]
    if value is None:
        return None
    if isinstance(value, list):
        return [str(pattern) for pattern in value]
    return [str(value)]


def _branch_admitted(config, branch):
    branches = _filter_patterns(config, "branches")
    if branches is not None and not _pattern_list_claims(branches, branch):
        return False
    ignore = _filter_patterns(config, "branches-ignore")
    if ignore is not None and _pattern_list_claims(ignore, branch):
        return False
    return True


def workflow_fires(workflow, branch):
    """Whether the workflow can fire for the branch, and whether it fires
    for the branch only through path filtered events, under FIX-5."""
    configs = _event_configs(workflow["triggers"])
    fires = False
    unfiltered = False
    for event in BRANCH_EVENTS:
        if event not in configs:
            continue
        config = configs[event]
        if not _branch_admitted(config, branch):
            continue
        fires = True
        if "paths" not in config and "paths-ignore" not in config:
            unfiltered = True
    return fires, fires and not unfiltered


def _job_forms(workflow_name, job):
    """The check names one job produces, as bare and workflow form pairs,
    one pair per matrix combination under FIX-2."""
    display = job["job_name"]
    combinations = job.get("combinations")
    suffixes = [None] if combinations is None else combinations
    forms = []
    for suffix in suffixes:
        bare = display if suffix is None else f"{display} ({suffix})"
        forms.append((bare, f"{workflow_name} / {bare}"))
    return forms


def _record_ran(record):
    return record["status"] == "completed" and record["conclusion"] in RAN_CONCLUSIONS


def _record_skipped(record):
    return record["status"] == "completed" and record["conclusion"] == "skipped"


def _last_run_date(ran_records):
    finishing = [
        record["completed_at"] or record["started_at"] for record in ran_records
    ]
    finishing = [stamp for stamp in finishing if stamp is not None]
    if not finishing:
        return None
    return max(finishing).date().isoformat()


def reconcile(contexts, workflows, records, branch):
    """One reconciled entry per required context, in protection order."""
    firing = []
    for workflow in workflows:
        fires, filtered_only = workflow_fires(workflow, branch)
        if fires:
            firing.append((workflow, filtered_only))

    entries = []
    for context in contexts:
        producers = []
        vouched = []
        for workflow, filtered_only in firing:
            for job_id in sorted(workflow["jobs"]):
                job = workflow["jobs"][job_id]
                for bare, full in _job_forms(workflow["workflow"], job):
                    if context == bare or context == full:
                        producers.append(
                            {
                                "workflow": workflow["workflow"],
                                "file": workflow["file"],
                                "job": job["job"],
                                "job_name": job["job_name"],
                                "filtered_only": filtered_only,
                                "has_if": job["has_if"],
                            }
                        )
                        vouched.append((workflow["workflow"], {bare, full}))
                        break
        producers.sort(key=lambda p: (p["file"], p["job"]))

        matched = [
            record
            for record in records
            if any(
                record["workflow_name"] == workflow_name and record["name"] in names
                for workflow_name, names in vouched
            )
        ]
        ran = [record for record in matched if _record_ran(record)]
        skipped = [record for record in matched if _record_skipped(record)]

        if not producers:
            status = "no_producer"
        elif ran:
            status = "alive"
        elif skipped:
            status = "skipped_only"
        else:
            status = "never_run"

        if producers:
            last_run_date = _last_run_date(ran)
        else:
            tail = context.rsplit(" / ", 1)[1] if " / " in context else context
            loose = [
                record
                for record in records
                if record["name"] in (context, tail) and _record_ran(record)
            ]
            last_run_date = _last_run_date(loose)

        notes = []
        if (
            status == "never_run"
            and producers
            and all(p["filtered_only"] for p in producers)
        ):
            notes.append(NOTE_PATH_FILTERED)
        if any(p["has_if"] for p in producers):
            notes.append(NOTE_IF_CONDITION)

        entries.append(
            {
                "context": context,
                "status": status,
                "producers": [
                    {
                        "workflow": p["workflow"],
                        "file": p["file"],
                        "job": p["job"],
                        "job_name": p["job_name"],
                    }
                    for p in producers
                ],
                "last_run_date": last_run_date,
                "notes": notes,
            }
        )
    return entries
