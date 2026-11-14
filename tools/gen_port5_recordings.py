#!/usr/bin/env python3
"""Regenerate the PORT5 explicit key and tag repair recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT5-6 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched except for the movements the amendment to
PORT5-6 in the PORT5 section states. The yaml_values case
standing as block scalar as an explicit key refuses flips to
the runtime's recorded reading of that document under the
name block scalar as an explicit key, the two unnamed
yaml_values cases standing from the first slice as port
stated refusals over an explicit pair and over the question
mark against the colon flip to their recorded readings, and
the corpus cases below are appended to the yaml_values
section by their families, the explicit keys first and the
tag repair behind them. The border cases the port keeps refused while the
runtime reading accepts them, the anchor and the tag and the
alias standing in explicit key position under PORT5-5, stand
as port stated refusals, in the standing form.

Nothing else in the recordings moves. The script asserts
that on every run, so a regeneration that would move a
standing recording outside the stated movement fails here
rather than in the suite.
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


# The corpus, by the families PORT5-6 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. Explicit keys in block mappings.
    ("explicit key reads its pair", "? key\n: value\n"),
    ("explicit pair beside compact pairs", "a: 1\n? b\n: 2\n"),
    ("explicit key spread over lines", "?\n  key\n: value\n"),
    ("explicit key with a block sequence value", "? key\n:\n  - 1\n  - 2\n"),
    ("explicit key with a block mapping value", "? key\n:\n  a: 1\n"),
    ("explicit key with a following lines scalar value", "? key\n:\n  value\n"),
    ("explicit null key", "? ~\n: value\n"),
    ("explicit int key", "? 42\n: value\n"),
    ("explicit date key", "? 2026-01-01\n: value\n"),
    ("explicit quoted key", "? \"a b\"\n: 1\n"),
    ("explicit key with no colon line reads null", "? foo\n"),
    ("explicit key with no colon line before a pair", "? foo\nbar: 1\n"),
    ("explicit merge key", "base: &b {x: 1}\n? <<\n: *b\n"),
    ("explicit merge key with a list of sources", "one: &o {a: 1}\ntwo: &t {b: 2}\n? <<\n: [*o, *t]\n"),
    ("duplicate explicit key", "? a\n: 1\nb: 0\n? a\n: 2\n"),
    ("question mark against the colon", "?: value\n"),
    ("block scalar folded explicit key", "? >\n  x\n: 1\n"),
    ("explicit key with a block scalar value", "? k\n: |\n  body\n"),
    ("explicit key in a sequence mapping", "- ? k\n  : v\n"),
    # 2. Explicit keys in flow mappings.
    ("flow explicit key", "{? key: value}\n"),
    ("flow explicit pair beside compact pairs", "{a: 1, ? b: 2}\n"),
    ("flow explicit key with no colon", "{? a}\n"),
    ("flow bare question mark", "{?}\n"),
    # 3. The refusals of the family.
    ("collection explicit key refuses", "? [a, b]\n: 1\n"),
    ("flow collection explicit key refuses", "{? [a]: 1}\n"),
    ("colon line with no pair open refuses", ": value\n"),
    ("dash opening as an explicit key refuses", "? - a\n: 1\n"),
    ("mapping as an explicit key refuses", "? a: 1\n: 2\n"),
    ("bare dash as an explicit key refuses", "? -\n: 1\n"),
    ("question line carrying its colon refuses", "? : v\n"),
    ("anchored explicit key refuses", "? &a k\n: 1\n"),
    ("tagged explicit key refuses", "? !!str k\n: 1\n"),
    ("alias explicit key refuses", "a: &x 1\n? *x\n: 2\n"),
    # 4. The tag repair, a tag converting its scalar from the
    # scalar's own text on the following lines path.
    ("str tag on following lines over an invalid date", "key: !!str\n  2026-13-45\n"),
    ("str tag on following lines over a valid date", "key: !!str\n  2026-01-01\n"),
    ("int tag on following lines over a date spelling", "key: !!int\n  2026-01-01\n"),
    ("timestamp tag on following lines over an invalid date", "key: !!timestamp\n  2026-13-45\n"),
    ("bool tag on following lines", "key: !!bool\n  yes\n"),
    ("float tag on following lines", "key: !!float\n  1.5\n"),
    ("str tag on following lines in a sequence", "- !!str\n  2026-13-45\n"),
    ("str tag standing alone at the root over an invalid date", "!!str\n2026-13-45\n"),
]

# The border cases the port keeps refused while the runtime
# reading accepts them, the anchor and the tag and the alias
# in explicit key position of PORT5-5. Their recordings stand
# as port stated refusals, in the standing form.
PORT_REFUSALS = {
    "anchored explicit key refuses",
    "tagged explicit key refuses",
    "alias explicit key refuses",
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
    standing = [json.dumps(case, sort_keys=True) for case in values]
    text_flips = {"? explicit\n: key\n", "?: v\n"}
    flipped = set()
    for index, case in enumerate(values):
        by_name = case.get("name") == "block scalar as an explicit key refuses"
        by_text = case.get("text") in text_flips
        if (by_name or by_text) and case.get("port_stated"):
            if by_name:
                case["name"] = "block scalar as an explicit key"
            case["expect"] = recorded(case["text"])
            if "ok" not in case["expect"]:
                raise SystemExit(f"the flip of {case.get('name')!r} does not read")
            del case["port_stated"]
            flipped.add(index)
    if len(flipped) != 3:
        raise SystemExit(f"the flip cases stand flipped already or missing: {len(flipped)}")
    for index, case in enumerate(values):
        if index not in flipped and json.dumps(case, sort_keys=True) != standing[index]:
            raise SystemExit("a standing yaml_values case would move outside the flip")
    standing_names = {case.get("name") for case in values}
    for name, text in CORPUS:
        if name in standing_names:
            raise SystemExit(f"corpus name {name!r} already stands in the recordings")
        expect, port_stated = corpus_expect(name, text)
        case = {"name": name, "text": text, "expect": expect}
        if port_stated:
            case["port_stated"] = True
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
