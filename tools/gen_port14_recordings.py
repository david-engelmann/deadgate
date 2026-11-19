#!/usr/bin/env python3
"""Regenerate the PORT14 merge standing completed recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT14-4 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched, the one case PORT14-4 names excepted.
That case, the port stated refusal the amendment to
PORT13-2 added, flips to the runtime's merged answer and
drops its port stated mark in this regeneration, as the
amendment discharging that limit states. The script
asserts both standings on every run, so a regeneration
that would move any other standing recording fails here
rather than in the suite.

The encoding conventions are the ninth slice's. A set is
encoded by its members' tagged forms sorted by the members'
repr spellings, the convention PORT9-5 states. A list whose
every member is a tuple is the reading of an ordered map or
a pairs node and is encoded as pairs, wherever it stands,
a key position among the standings, and a plain list stands
as a list even standing as a pair's key.

The one case the corpus carries as a port stated refusal
is the set standing as the pair value of an alias merge
key, the limit PORT14-5 states, in the standing form the
third slice's generator sets. Its document is the limit
that obligation names, and its recorded answer is the
port's contracted refusal, the runtime reading of the
document standing measured in the contract beside the
obligation.
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


# The corpus, by the families PORT14-4 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. The alias merge family, the alias standing as a key
    # to an anchored merge key over every standing of its
    # pair value.
    ("alias key to an anchored merge key merging a mapping value at an explicit key", "&m <<: {b: 2}\n? *m\n: {c: 3}"),
    ("alias key to an anchored merge key with a scalar value refusing", "&m <<: {b: 2}\n? *m\n: v"),
    ("alias key to an anchored merge key merging a mapping value at a compact key", "&m <<: {b: 2}\n*m: {c: 3}"),
    ("alias key to an anchored merge key merging a mapping value in flow", "{&m <<: {b: 2}, *m: {c: 3}}"),
    ("alias key to an anchored merge key merging a list of mappings", "&m <<: {b: 2}\n? *m\n: [{c: 3}]"),
    ("alias key to an anchored merge key merging an alias to a mapping", "src: &s {c: 3}\n&m <<: {b: 2}\n? *m\n: *s"),
    ("merge tagged key standing as an explicit key merging through an alias key", "? &m !!merge <<\n: {b: 2}\n? *m\n: {c: 3}"),
    ("alias key to an anchored merge key refusing a list carrying a scalar", "&m <<: {b: 2}\n? *m\n: [{c: 3}, 4]"),
    ("alias key to an anchored merge key refusing an empty value", "&m <<: {b: 2}\n? *m\n:"),
    ("alias key to an anchored merge key with a set standing as its value", "&m <<: {b: 2}\n? *m\n: !!set {c}"),
    ("alias key to an anchored merge key merging twice in one mapping", "&m <<: {b: 2}\n? *m\n: {c: 3}\n? *m\n: {d: 4}"),
    ("alias key to an anchored merge key merging a value carrying its own merge", "&m <<: {b: 2}\n? *m\n: {<<: {e: 5}, c: 3}"),
    ("alias key to an anchored merge key merging in a nested mapping", "outer:\n  &m <<: {b: 2}\n  ? *m\n  : {c: 3}"),
    ("alias key to an anchored merge key merging a value carrying its own anchor", "&m <<: {b: 2}\n? *m\n: &v {c: 3}"),
    ("alias key to an anchored merge key merging a block mapping behind the colon", "&m <<: {b: 2}\n? *m\n:\n  c: 3"),
    ("alias key standing as a set member refusing an empty value", "&m <<: {b: 2}\nt: !!set\n  ? *m\n"),
    ("merge key standing as a pair in a set", "!!set\n? <<\n: {b: 2}"),
    ("anchored merge key defined in value position refusing at a mapping value", "a: &m <<\n? *m\n: {c: 3}"),
    ("anchored merge key defined in value position refusing at a scalar value", "a: &m <<\n? *m\n: v"),
    ("quoted merge spelling standing through an alias key with a mapping value", "a: &m \"<<\"\n? *m\n: {c: 3}"),
    # 2. The merge tagged family, a key standing under the
    # merge tag with a text that is not the spelling.
    ("merge tagged key with other text refusing a scalar value through an alias key", "&m !!merge foo: {b: 2}\n? *m\n: v"),
    ("merge tagged key with other text merging a mapping value through an alias key", "&m !!merge foo: {b: 2}\n? *m\n: {c: 3}"),
    ("merge tagged key with other text merging at a compact key", "&m !!merge foo: {b: 2}\n*m: {c: 3}"),
    ("merge tagged key with other text merging in flow", "{&m !!merge foo: {b: 2}, *m: {c: 3}}"),
    ("merge tagged key with other text standing as a direct key", "!!merge foo: {b: 2}"),
    ("merge tagged key with a boolean text merging through an alias key", "&m !!merge yes: {b: 2}\n? *m\n: {c: 3}"),
    ("merge tagged key with other text merging a list of mappings through an alias key", "&m !!merge foo: {b: 2}\n? *m\n: [{c: 3}]"),
    # 3. The flow sequence pair family, the anchored merge
    # key standing as a pair inside a flow sequence and its
    # borders.
    ("anchored merge key standing as a flow sequence pair merging", "[&m <<: {b: 2}]"),
    ("merge key standing as a flow sequence pair merging", "[<<: {b: 2}]"),
    ("merge tagged key standing as a flow sequence pair merging", "[!!merge <<: {b: 2}]"),
    ("anchored merge key standing as a flow sequence pair beside a plain pair", "[a: 1, &m <<: {b: 2}]"),
    ("anchored merge key standing as a flow sequence pair nested in a flow mapping", "{x: [&m <<: {b: 2}]}"),
    ("anchored merge key standing as a block sequence pair merging", "- &m <<: {b: 2}"),
    ("merge key standing as a block sequence pair merging", "- <<: {b: 2}"),
    ("alias key standing as a flow sequence pair refusing", "&m <<: {b: 2}\n[*m: {c: 3}]"),
    ("anchored merge key standing as a flow sequence pair beside a scalar entry merging", "[&m <<: {b: 2}, z]"),
    ("anchored quoted merge spelling standing as a flow sequence pair", "[&m \"<<\" : {b: 2}]"),
]


# The case the slice flips, the port stated refusal the
# amendment to PORT13-2 added. It regenerates from the
# runtime reading and drops its port stated mark, as
# PORT14-4 and the amendment discharging that limit state.
FLIP_NAME = "alias key to an anchored merge key with a mapping value refuses"


# The border case the port keeps refused while the runtime
# reading reads it, the standing limit PORT14-5 names. Its
# recording stands as a port stated refusal, in the
# standing form.
PORT_REFUSALS = {
    "alias key to an anchored merge key with a set standing as its value",
}


def corpus_expect(name, text):
    if name in PORT_REFUSALS:
        return {"err": "port refusal"}, True
    return recorded(text), False


def main():
    if yaml.__version__ != "6.0.1":
        raise SystemExit(f"PyYAML stands at {yaml.__version__}, the corpus wants 6.0.1")
    document = json.loads(RECORDINGS.read_text(encoding="utf-8"))
    sections = document["sections"]
    before = {name: json.dumps(cases, sort_keys=True) for name, cases in sections.items()}
    census_before = {name: len(cases) for name, cases in sections.items()}

    values = sections["yaml_values"]
    standing_by_name = {case.get("name"): case for case in values}

    # The one standing case the slice names flips to the
    # runtime's merged answer and drops its mark.
    if FLIP_NAME not in standing_by_name:
        raise SystemExit(f"the case this slice flips, {FLIP_NAME!r}, does not stand")
    flip_case = standing_by_name[FLIP_NAME]
    flipped = {"name": FLIP_NAME, "text": flip_case["text"], "expect": recorded(flip_case["text"])}
    if json.dumps(flip_case, sort_keys=True) == json.dumps(flipped, sort_keys=True):
        raise SystemExit("the flipped case already stands at the runtime answer")
    values[values.index(flip_case)] = flipped

    for name, text in CORPUS:
        expect, port_stated = corpus_expect(name, text)
        case = {"name": name, "text": text, "expect": expect}
        if port_stated:
            case["port_stated"] = True
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
        wanted = flipped if case.get("name") == FLIP_NAME else case
        if json.dumps(values[index], sort_keys=True) != json.dumps(wanted, sort_keys=True):
            raise SystemExit("a standing yaml_values case would move")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    main()
