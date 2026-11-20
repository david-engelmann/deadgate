#!/usr/bin/env python3
"""Regenerate the PORT15 separator standing completed recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT15-7 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched, the six cases PORT15-7 names excepted.
Those cases, the port stated readings the third slice's
limits paragraph stated for documents carrying the line
separator U+2028 or the paragraph separator U+2029, flip
to the runtime's separator standing answers and drop
their port stated marks in this regeneration, as the
notes discharging those limits state. The script asserts
both standings on every run, so a regeneration that would
move any other standing recording fails here rather than
in the suite.

The encoding conventions are the ninth slice's. A set is
encoded by its members' tagged forms sorted by the members'
repr spellings, the convention PORT9-5 states. A list whose
every member is a tuple is the reading of an ordered map or
a pairs node and is encoded as pairs, wherever it stands,
a key position among the standings, and a plain list stands
as a list even standing as a pair's key.

No case in the corpus stands as a port stated refusal.
The one limit the slice keeps, the root plain standing of
PORT15-4, is held by its pin in the separators target and
carries no recording here, and the flow plain key refusal
of PORT15-5 is likewise held by its pins.
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


# The corpus, by the families PORT15-7 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. The quoted family, the separators standing inside
    # quoted spans and the escapes standing as content.
    ("line separator inside a double quoted scalar stands as itself", "\"a b\""),
    ("paragraph separator inside a single quoted scalar stands as itself", "'a b'"),
    ("line separator and a line feed inside a double quoted span stand in order", "\"a \nb\""),
    ("line feed and a line separator inside a double quoted span fold to the separator", "\"a\n b\""),
    ("line separator and paragraph separator inside a double quoted span stand in order", "\"a  b\""),
    ("blanks beside a line separator inside a double quoted span are trimmed", "\"a   b\""),
    ("escaped line separator inside a double quoted scalar stands as content", "\"a\\Lb\""),
    ("escaped paragraph separator inside a double quoted scalar stands as content", "\"a\\Pb\""),
    ("line separator inside a single quoted mapping value stands as itself", "a: 'x y'"),
    ("line separator and two line feeds inside a double quoted span stand in order", "\"a \n\nb\""),
    # 2. The plain family, the separators standing as
    # content in block plain readings.
    ("block plain value with a line separator and a space behind it", "a: x  y"),
    ("block plain value with a paragraph separator at the end of its line", "a: x \n y\n"),
    ("block plain value with a line feed and a line separator between its chunks", "a: x\n  y\n"),
    ("block plain value with a separator standing after a blank run", "a: x\n\n  y"),
    ("block plain value with a blank before a separator at the end of its line", "a: x   y"),
    ("block plain value with a separator standing before a sibling pair", "a: x  y\nb: 2"),
    ("block plain value with a paragraph separator and a space behind it", "a: x  y"),
    ("sequence entry plain value with a line separator and a space behind it", "- x  y"),
    # 3. The flow family, the separators standing as
    # content and as separation in the flow readings.
    ("flow plain value with a line separator standing as content", "{a: x y}"),
    ("flow plain value with blanks beside a line separator trimmed", "{a: x   y}"),
    ("line separator standing as separation in a flow sequence", "[x, y]"),
    ("flow plain value with a separator and a line feed standing after it", "{a: x \ny}"),
    ("comment inside a flow mapping ending at a line separator", "{a: 1, # c b: 2}"),
    ("flow plain value with a paragraph separator standing as content", "{a: x y}"),
    # 4. The block family, the separators standing in the
    # literal and folded block readings.
    ("literal block with a line separator at the end of a content line", "a: |\n  one \n  two\n"),
    ("folded block with a line separator at the end of a content line", "a: >\n  one \n  two\n"),
    ("literal block with a line separator standing alone on a line", "a: |\n  one\n   \n  two\n"),
    ("folded block with a line separator standing alone on a line", "a: >\n  one\n   \n  two\n"),
    ("literal block with content standing behind a line separator", "a: |\n  one   two\n"),
    ("block under keep with a paragraph separator at the end of its content", "a: |+\n  one \n"),
    ("block under strip with a paragraph separator at the end of its content", "a: |-\n  one \n"),
    ("literal block with a line separator standing before its first content line", "a: |\n   \n  one\n"),
    ("block under keep whose lines are only separator spaces lines", "a: |+\n   \n   \nb: 1\n"),
    ("folded block with a more indented line separator line", "a: >\n  one\n     \n  two\n"),
]


# The cases the slice flips, the port stated readings the
# third slice's limits paragraph stated. Each regenerates
# from the runtime reading and drops its port stated mark,
# as PORT15-7 and the notes discharging those limits state.
FLIP_NAMES = [
    "block line separator inside literal content",
    "block paragraph separator at the end under keep",
    "block line separator inside folded content",
    "line separator inside a span folds as a break",
    "paragraph separator inside a span folds as a break",
    "line separator in a flow span folds as a break",
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

    # The standing cases the slice names flip to the
    # runtime's separator standing answers and drop
    # their marks.
    flipped_by_name = {}
    for flip_name in FLIP_NAMES:
        if flip_name not in standing_by_name:
            raise SystemExit(f"the case this slice flips, {flip_name!r}, does not stand")
        flip_case = standing_by_name[flip_name]
        flipped = {
            "name": flip_name,
            "text": flip_case["text"],
            "expect": recorded(flip_case["text"]),
        }
        if json.dumps(flip_case, sort_keys=True) == json.dumps(flipped, sort_keys=True):
            raise SystemExit(f"the flipped case {flip_name!r} already stands at the runtime answer")
        values[values.index(flip_case)] = flipped
        flipped_by_name[flip_name] = flipped

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
        wanted = flipped_by_name.get(case.get("name"), case)
        if json.dumps(values[index], sort_keys=True) != json.dumps(wanted, sort_keys=True):
            raise SystemExit("a standing yaml_values case would move")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    main()
