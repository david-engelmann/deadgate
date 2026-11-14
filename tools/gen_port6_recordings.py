#!/usr/bin/env python3
"""Regenerate the PORT6 key property recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT6-6 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched except for the movements the amendment to
PORT6-6 in the PORT6 section states. The six yaml_values
cases standing as port stated refusals over properties in
key position flip to the runtime's recorded readings, the
five named explicit key cases dropping the refusal word
from their names as they flip and the unnamed flow case
taking the name anchor in front of a flow mapping key, and
the corpus cases below are appended to the yaml_values
section by their families. Nothing else in the recordings
moves. The script asserts that on every run, so a
regeneration that would move a standing recording outside
the stated movement fails here rather than in the suite.
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
    if isinstance(value, list):
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


# The corpus, by the families PORT6-6 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. Properties in front of a compact block key.
    ("anchor in front of a block key", "&a key: v\n"),
    ("anchor in front of a block key on a later pair", "a: 1\n&x b: 2\n"),
    ("anchor in front of a quoted key", "&a 'key': v\n"),
    ("anchor on a block key binds the key", "&a k: v\nother: *a\n"),
    ("str tag in front of a block key", "!!str 5: v\n"),
    ("int tag over a quoted key", "!!int \"5\": v\n"),
    ("float tag in front of a block key", "!!float '1.5': v\n"),
    ("bool tag in front of a block key", "!!bool 'yes': v\n"),
    ("null tag in front of a block key", "!!null 'x': v\n"),
    ("both properties in front of a block key", "!!str &a 5: v\nother: *a\n"),
    ("both properties in front of a block key swapped", "&a !!str 5: v\nother: *a\n"),
    ("anchor on the merge spelling is refused", "&a <<: v\n"),
    # 2. Properties in the flow key positions.
    ("anchor in front of a flow key after a comma", "{a: 1, &b k: v, x: *b}\n"),
    ("tag in front of a flow key", "{!!str 5: v}\n"),
    ("flow explicit key with an anchor", "{? &a k: v, x: *a}\n"),
    ("flow explicit key with a tag", "{? !!str 5: v}\n"),
    ("anchored pair key in a flow sequence", "[{&a k: v}, *a]\n"),
    ("tagged pair key in a flow sequence", "[{!!str 5: v}]\n"),
    # 3. Properties in the explicit key positions.
    ("property in front of a block scalar explicit key", "? !!str |\n  k\n: v\n"),
    ("explicit key anchor binds the key", "? &a k\n: v\nother: *a\n"),
    # 4. Aliases standing as keys.
    ("alias standing as a block key", "base: &a name\nother:\n  *a : v\n"),
    ("alias standing as a flow key", "defs: &a name\nm: {*a : 1}\n"),
    ("alias standing as a pair key in a flow sequence", "d: &x name\ns: [{*x : v}]\n"),
    ("alias to a collection standing as a key refuses", "base: &a [1, 2]\nother:\n  *a : v\n"),
    ("undefined alias standing as a key refuses", "m:\n  *nope : 1\n"),
    # 5. Conversions and duplicates.
    ("duplicate formed by conversion", "5: a\n!!int \"5\": b\n"),
    ("str tag key beside the plain key", "5: a\n!!str 5: b\n"),
    # 6. The borders the slice keeps.
    ("binary tag in key position refuses", "!!binary k: v\n"),
    ("seq tag on a scalar key refuses", "!!seq k: v\n"),
]

FLIP_NAMES = {
    "anchored explicit key refuses": "anchored explicit key",
    "tagged explicit key refuses": "tagged explicit key",
    "alias explicit key refuses": "alias explicit key",
    "anchored key on following lines refuses": "anchored key on following lines",
    "alias key on following lines refuses": "alias key on following lines",
}


def main():
    if yaml.__version__ != "6.0.1":
        raise SystemExit(f"PyYAML stands at {yaml.__version__}, the corpus wants 6.0.1")
    document = json.loads(RECORDINGS.read_text(encoding="utf-8"))
    sections = document["sections"]
    before = {name: json.dumps(cases, sort_keys=True) for name, cases in sections.items()}
    census_before = {name: len(cases) for name, cases in sections.items()}

    values = sections["yaml_values"]
    standing = [json.dumps(case, sort_keys=True) for case in values]
    flipped = set()
    for index, case in enumerate(values):
        name = case.get("name")
        by_name = name in FLIP_NAMES
        by_text = case.get("text") == "{&a k: x}\n"
        if (by_name or by_text) and case.get("port_stated"):
            if by_name:
                case["name"] = FLIP_NAMES[name]
            else:
                case["name"] = "anchor in front of a flow mapping key"
            case["expect"] = recorded(case["text"])
            if "ok" not in case["expect"]:
                raise SystemExit(f"the flip of {case.get('name')!r} does not read")
            del case["port_stated"]
            flipped.add(index)
    if len(flipped) != 6:
        raise SystemExit(f"the flip cases stand flipped already or missing: {len(flipped)}")
    for index, case in enumerate(values):
        if index not in flipped and json.dumps(case, sort_keys=True) != standing[index]:
            raise SystemExit("a standing yaml_values case would move outside the flips")
    standing_names = {case.get("name") for case in values}
    for name, text in CORPUS:
        if name in standing_names:
            raise SystemExit(f"corpus name {name!r} already stands in the recordings")
        case = {"name": name, "text": text, "expect": recorded(text)}
        values.append(case)

    census_after = {name: len(cases) for name, cases in sections.items()}
    for name, cases in sections.items():
        if name == "yaml_values":
            continue
        if json.dumps(cases, sort_keys=True) != before[name]:
            raise SystemExit(f"section {name} would move outside the stated movement")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    main()
