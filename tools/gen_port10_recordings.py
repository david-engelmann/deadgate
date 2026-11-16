#!/usr/bin/env python3
"""Regenerate the PORT10 pairs key recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT10-6 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched. This slice moves no standing recording,
the pairs key family standing unrecorded in the standing
batteries, and the corpus cases below are appended to the
yaml_values section. The script asserts that on every run,
so a regeneration that would move a standing recording
fails here rather than in the suite.

The encoding conventions are the ninth slice's. A set is
encoded by its members' tagged forms sorted by the members'
repr spellings, the convention PORT9-5 states. A list whose
every member is a tuple is the reading of an ordered map or
a pairs node and is encoded as pairs, wherever it stands,
a key position among the standings, and a plain list stands
as a list even standing as a pair's key. The empty ordered
map reads on the runtime side as the empty list and stands
in the slice's pins rather than here, its recorded spelling
being the empty list either way.
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


# The corpus, by the families PORT10-6 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. A pair's key standing as a container.
    ("pairs entry whose key is a sequence", "a: !!pairs\n  - [1, 2]: v\n"),
    ("pairs entry whose key is a mapping", "a: !!pairs\n  - {x: 1}: v\n"),
    ("pairs entry whose key is a set", "a: !!pairs\n  - !!set {p, q}: v\n"),
    ("pairs entry whose key is a list of pairs", "a: !!pairs\n  - !!omap [x: 1]: v\n"),
    ("ordered map entry whose key is a sequence", "a: !!omap\n  - [1, 2]: v\n"),
    ("pairs in flow form with a sequence key", "a: !!pairs [{[1, 2]: v}]\n"),
    ("pairs entry whose key is a block sequence", "a: !!pairs\n  - ? - 1\n      - 2\n    : v\n"),
    ("pairs entry whose key is a block mapping", "a: !!pairs\n  - ? x: 1\n    : v\n"),
    # 2. The widened keys standing recursively and beside
    # each other.
    ("pairs key holding a list of pairs with a sequence key", "a: !!pairs\n  - !!omap [[9]: deep]: top\n"),
    ("pairs with duplicate sequence keys", "a: !!pairs\n  - [1, 2]: x\n  - [1, 2]: y\n"),
    ("pairs entry whose key is an empty sequence", "a: !!pairs\n  - []: v\n"),
    ("pairs entry whose key is an empty mapping", "a: !!pairs\n  - {}: v\n"),
    ("pairs key that is a mapping with a sequence value", "a: !!pairs\n  - {x: [1, 2]}: v\n"),
    ("pairs key that is a mapping with a pairs value", "a: !!pairs\n  - {x: !!pairs [y: 2]}: v\n"),
    ("pairs entry whose key is a sequence carrying a null", "a: !!pairs\n  - [~, 2]: v\n"),
    ("an alias of a sequence as a pairs key", "defs: &k [7, 8]\nuse: !!pairs\n  - *k: v\n"),
    ("a merged mapping as a pairs key", "src: &m {a: 1}\nuse: !!pairs [{? {<<: *m, b: 2}: v}]\n"),
    # 3. The borders PORT10-2 keeps.
    ("pairs value carrying a mapping with a sequence key", "a: !!pairs\n  - k: {[1, 2]: v}\n"),
    ("ordered map entry with two pairs and a container key", "a: !!omap\n  - {[1]: x, y: 2}\n"),
    ("a set as a pairs key with an unhashable member", "a: !!pairs\n  - !!set {? !!pairs [x: y]}: v\n"),
    ("pairs standing as a key in block form", "? !!pairs [x: 1]\n: 2\n"),
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
