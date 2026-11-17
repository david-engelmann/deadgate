#!/usr/bin/env python3
"""Regenerate the PORT13 merge spelling through an alias key recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT13-4 names, and writes the
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


# The corpus, by the families PORT13-4 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. The reading at an explicit key, the anchored scalar
    # carrying the spelling under the string tag in each of
    # its spellings.
    ("quoted merge spelling at an explicit key through an alias", "a: &m \"<<\"\n? *m\n: v\n"),
    ("str tagged merge spelling at an explicit key through an alias", "a: &m !!str <<\n? *m\n: v\n"),
    ("literal spelled merge spelling at an explicit key through an alias", "a: &m |-\n  <<\n? *m\n: v\n"),
    ("folded spelled merge spelling at an explicit key through an alias", "a: &m >-\n  <<\n? *m\n: v\n"),
    ("single quoted merge spelling at an explicit key through an alias", "a: &m '<<'\n? *m\n: v\n"),
    # 2. The reading at the wider alias key standings.
    ("quoted merge spelling at a compact block key through an alias", "a: &m \"<<\"\n*m: v\n"),
    ("quoted merge spelling at a flow compact key through an alias", "a: &m \"<<\"\nb: {*m: v}\n"),
    ("quoted merge spelling at a flow explicit key through an alias", "a: &m \"<<\"\nb: {? *m: v}\n"),
    ("quoted merge spelling nested inside flow collections through an alias", "a: &m \"<<\"\nb: {x: {*m: v}}\n"),
    ("quoted merge spelling through an alias key beside a direct merge", "src: &s {p: 1}\na: &m \"<<\"\nouter:\n  <<: *s\n  *m: v\n"),
    ("quoted merge spelling through an alias key duplicated by a direct string key", "a: &m \"<<\"\n? *m\n: first\n\"<<\": second\n"),
    ("anchored quoted merge spelling standing as a key through an alias", "&m \"<<\": 1\n? *m\n: v\n"),
    ("anchored str tagged merge spelling standing as a key through an alias", "&m !!str <<: 1\n? *m\n: v\n"),
    # 3. The standings that do not move.
    ("quoted merge spelling through an alias in value position", "a: &m \"<<\"\nb: *m\n"),
    ("str tagged scalar at an explicit key through an alias", "a: &t !!str yes\n? *t\n: v\n"),
    ("direct merge key with an anchor in front of it", "a: 1\n&m <<: {b: 2}\n"),
    ("direct merge key with the merge tag in front of it", "a: 1\n!!merge <<: {b: 2}\n"),
    ("direct merge key with an anchor and the merge tag in front of it", "a: 1\n&m !!merge <<: {b: 2}\n"),
    # 4. The borders PORT13-2 keeps.
    ("plain anchored merge spelling refusing at an alias key", "a: &m <<\n? *m\n: v\n"),
    ("merge tagged anchored merge spelling refusing", "a: &m !!merge <<\n"),
    ("merge tagged merge spelling in value position refusing", "x: !!merge <<\n"),
    ("plain merge spelling in entry position refusing", "- <<\n"),
    ("anchored plain merge spelling in entry position refusing", "- &m <<\n"),
    ("anchored set refusing at an alias key", "a: &s !!set {x, y}\n? *s\n: v\n"),
    ("anchored list of pairs refusing at an alias key", "a: &p !!omap [x: 1]\n? *p\n: v\n"),
    ("alias key to an anchored plain merge key refusing", "&m <<: {b: 2}\n? *m\n: v\n"),
    ("alias key to an anchored merge tagged key refusing", "&m !!merge <<: {b: 2}\n? *m\n: v\n"),
    ("alias compact key to an anchored plain merge key refusing", "&m <<: {b: 2}\n*m: v\n"),
    ("alias key to an anchored plain merge key in flow refusing", "{&m <<: {b: 2}, *m: v}\n"),
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
