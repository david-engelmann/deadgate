#!/usr/bin/env python3
"""Regenerate the PORT12 hash in plain scalar recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT12-4 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched. This slice moves no standing recording,
and the corpus cases below are appended to the yaml_values
section. The script asserts that on every run, so a
regeneration that would move a standing recording fails
here rather than in the suite.

The encoding conventions are the ninth slice's. A set is
encoded by its members' tagged forms sorted by the members'
repr spellings, the convention PORT9-5 states. A list whose
every member is a tuple is the reading of an ordered map or
a pairs node and is encoded as pairs, wherever it stands,
a key position among the standings, and a plain list stands
as a list even standing as a pair's key.
"""

import datetime
import json
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
RECORDINGS = ROOT / "tests" / "conformance" / "port1_cases.json"

EPOCH = datetime.datetime(1970, 1, 1)


def epoch_micros(value):
    delta = value - EPOCH
    return (
        (delta.days * 86400 + delta.seconds) * 1000000 + delta.microseconds
    )


def tag(value):
    """The tagged form the conformance harness compares."""
    if value is None:
        return {"t": "null"}
    if isinstance(value, bool):
        return {"t": "bool", "v": value}
    if isinstance(value, int):
        return {"t": "int", "v": str(value)}
    if isinstance(value, float):
        if value != value:
            word = "nan"
        elif value == float("inf"):
            word = "inf"
        elif value == float("-inf"):
            word = "-inf"
        else:
            word = repr(value)
        return {"t": "float", "v": word}
    if isinstance(value, datetime.datetime):
        offset = value.utcoffset()
        if value.tzinfo is not None and offset is not None:
            utc = value.astimezone(datetime.timezone.utc).replace(tzinfo=None)
            return {
                "t": "aware",
                "micros": str(epoch_micros(utc)),
                "offset": int(offset.total_seconds()),
            }
        return {"t": "naive", "micros": str(epoch_micros(value))}
    if isinstance(value, datetime.date):
        return {"t": "date", "v": value.isoformat()}
    if isinstance(value, str):
        if all(0x20 <= ord(ch) <= 0x7E for ch in value):
            return {"t": "str", "v": value}
        return {"t": "strcp", "v": [ord(ch) for ch in value]}
    if isinstance(value, bytes):
        return {"t": "bytes", "v": list(value)}
    if isinstance(value, (set, frozenset)):
        members = sorted(value, key=repr)
        return {"t": "set", "v": [tag(member) for member in members]}
    if isinstance(value, list):
        if value and all(isinstance(item, tuple) for item in value):
            return {
                "t": "pairs",
                "v": [[tag(key), tag(item)] for key, item in value],
            }
        return {"t": "list", "v": [tag(item) for item in value]}
    if isinstance(value, dict):
        return {
            "t": "obj",
            "v": [[tag(key), tag(item)] for key, item in value.items()],
        }
    raise TypeError(f"no tag for {type(value)!r}")


def recorded(text):
    """The runtime answer for a document, as an expect field.

    A conversion raising outside the yaml error family stands
    recorded under its exception's type name, as the contract
    states, and never crashes the recording.
    """
    try:
        value = yaml.safe_load(text)
        return {"ok": tag(value)}
    except Exception as error:
        return {"err": type(error).__name__}


# The corpus, by the families PORT12-4 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. A hash standing inside a flow plain scalar, in the
    # positions PORT12-1 names.
    ("flow plain scalar carrying a hash", "[a#b, x]\n"),
    ("hash scalar as a flow mapping value", "a: [a#b, x]\n"),
    ("hash scalar as a flow key", "{a#b: 1}\n"),
    ("hash at a plain scalar's end", "[a#, x]\n"),
    ("hash inside a plain scalar spanning words", "[a# b, x]\n"),
    ("hashes across a flow mapping", "{a#b: c#d}\n"),
    ("a colon inside a flow plain scalar carrying a hash", "[a:#b]\n"),
    ("hash carrying scalars across a flow mapping", "{k: a#b, k2: c#d}\n"),
    ("doubled hashes inside a flow plain scalar", "[a##b, x]\n"),
    ("hash inside a flow mapping value", "{k: b#c}\n"),
    # 2. The reading standing in the wider standings.
    ("flow collection spanning lines carrying a hash", "a: [[x, a#b],\n  y]\n"),
    ("nested flow collection carrying a hash", "[[a#b], x]\n"),
    ("hash scalar beside a quoted scalar", "['quoted', a#b]\n"),
    ("comment after a hash carrying flow collection", "[a#b, x] # comment\n"),
    ("hash scalar standing as a pairs key", "a: !!pairs\n  - [a#b, x]: v\n"),
    ("hash scalar behind an anchor in flow", "[&a a#b, x]\n"),
    ("several hash scalars in one collection", "[a#b, c#d, x]\n"),
    # 3. The borders PORT12-2 keeps.
    ("comment at an entry start inside a flow collection", "[a, #b\n x]\n"),
    ("comment after a blank inside a flow entry", "[a #b\n, x]\n"),
    ("comment swallowing a closing bracket", "[a #b, x]\n"),
    ("unterminated flow collection carrying a hash", "[a#b, x\n"),
    ("hash at the start of a line inside a flow collection", "[a,\n#b\nx]\n"),
    ("bracket standing inside a comment", "[a # [\n, x]\n"),
]


def main():
    if yaml.__version__ != "6.0.1":
        raise SystemExit(f"PyYAML stands at {yaml.__version__}, the corpus wants 6.0.1")
    document = json.loads(RECORDINGS.read_text(encoding="utf-8"))
    sections = document["sections"]
    before = {name: json.dumps(cases, sort_keys=True) for name, cases in sections.items()}
    census_before = {name: len(cases) for name, cases in sections.items()}

    values = sections["yaml_values"]
    standing_by_name = {case.get("name"): case for case in values}
    for name, text in CORPUS:
        case = {"name": name, "text": text, "expect": recorded(text)}
        if name in standing_by_name:
            if json.dumps(standing_by_name[name], sort_keys=True) != json.dumps(
                case, sort_keys=True
            ):
                raise SystemExit(f"standing case {name!r} would move on a re-run")
            continue
        values.append(case)

    census_after = {name: len(cases) for name, cases in sections.items()}
    for name, cases in sections.items():
        if name == "yaml_values":
            continue
        if json.dumps(cases, sort_keys=True) != before[name]:
            raise SystemExit(f"section {name} would move outside the stated movement")
    standing_values = json.loads(before["yaml_values"])
    for index, case in enumerate(standing_values):
        if json.dumps(values[index], sort_keys=True) != json.dumps(case, sort_keys=True):
            raise SystemExit("a standing yaml_values case would move")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    main()
