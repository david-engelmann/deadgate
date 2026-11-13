#!/usr/bin/env python3
"""Regenerate the PORT4 anchor, alias, merge, and tag recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT4-10 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched except for the movement PORT4-10 states.
The seven standing yaml_values anchor and tag cases recorded
as port stated refusals flip to their recorded values, the
workflows anchor case flips to a recorded workflow that reads
whole, the corpus cases below are appended to the yaml_values
section by their families, and the yaml_strings section gains
the string forms of a merged mapping, of an aliased list used
twice, of a tagged timestamp value, and of a tagged string
document. The document past the expansion bound stands as a
port stated refusal, its answer the port's border and not a
runtime answer.

Where a conversion raises an error outside the yaml error
family, the exception's type name stands recorded in the
error form, as PORT4-10 states. The binary and set and omap
and pairs shapes never reach the tag function, since their
values carry no tagged form here. The corpus carries no such
case as a recorded answer, and this script refuses to write
one.

Nothing else in the recordings moves. The script asserts
that on every run, so a regeneration that would move a
standing recording outside the stated movement fails here
rather than in the suite.
"""

import datetime
import json
import sys
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


# The flips, the standing yaml_values cases PORT4-10 names,
# five found by their text and the two border recordings by
# their names.
TEXT_FLIPS = {
    "a: &anchor 1\n",
    "a: !!str 5\n",
    "merging:\n  base: &b\n    x: 1\n  child:\n    <<: *b\n    y: 2\n",
    "on: [&a push]\n",
    "{a: &x 1, b: *x}\n",
}
NAME_FLIPS = {
    "block head behind an anchor refuses",
    "block head behind a tag refuses",
}

# The border case the port refuses while the runtime reading
# accepts it, standing as a port stated refusal in the
# standing form.
PORT_REFUSALS = {
    "document past the expansion bound refuses",
}

# Tag spellings whose values carry no tagged form here. A
# corpus case carrying one must stand as a port stated
# refusal, never as a recorded answer.
BORDER_TAG_SPELLINGS = ("!!binary", "!!set", "!!omap", "!!pairs")


def corpus_expect(name, text):
    if name in PORT_REFUSALS:
        return {"err": "port refusal"}, True
    for spelling in BORDER_TAG_SPELLINGS:
        if spelling in text:
            raise SystemExit(
                f"corpus case {name!r} carries a border tag spelling "
                "as a recorded answer"
            )
    return recorded(text), False


def expansion_document(count):
    aliases = ", ".join(["*x"] * count)
    return f"a: &x 1\nb: [{aliases}]\n"


# The corpus, by the families PORT4-10 names. Each entry is a
# name and a document. The answers are recorded, never set
# down here.
CORPUS = [
    # Anchor placement.
    ("anchor on a quoted scalar", "a: &x 'hi'\nb: *x\n"),
    ("anchor at the root on a scalar", "&x hello\n"),
    ("anchor on a mapping value flow mapping", "a: &x {k: 1}\nb: *x\n"),
    ("anchor behind a document opener", "--- &x [1, 2]\n"),
    ("anchor on a dash line in front of a block mapping", "- &a key: v\n- *a\n"),
    ("anchor alone on a dash line binds the whole mapping", "- &a\n  key: v\n- *a\n"),
    ("anchor in front of a flow collection on a dash line", "- &a [1, 2]\n- *a\n"),
    (
        "anchor alone at the end of a mapping line binds the indented scalar",
        "a: &x\n  hello\nb: *x\n",
    ),
    (
        "anchor on an empty value binds the sequence at the key indent",
        "a: &x\n- p\n- q\nb: *x\n",
    ),
    ("anchor with a comment before its nested block", "a: &x # c\n  k: 1\nb: *x\n"),
    ("anchors inside a flow collection used after it", "a: [&x 1, &y 2]\nb: [*x, *y]\n"),
    # Alias resolution.
    ("alias used twice", "a: &x 5\nb: *x\nc: *x\n"),
    ("alias of a null", "a: &x\nb: *x\n"),
    ("alias of a block scalar", "a: &x |\n  hi\nb: *x\n"),
    ("alias of an anchored mapping inside a flow sequence", "a: [&x {k: 1}, *x]\n"),
    ("duplicate anchor refuses", "a: &x 1\nb: &x 2\n"),
    ("alias of a differently cased anchor refuses", "a: &X 1\nb: *x\n"),
    # Merge keys.
    ("merge key standing first", "a: &s {x: 1, y: 1}\nb:\n  <<: *s\n  y: 2\n"),
    ("merge key standing last", "a: &s {x: 1, y: 1}\nb:\n  y: 2\n  <<: *s\n"),
    (
        "merge list earlier listed source wins",
        "a: &a {x: 1, y: 1}\nb: &b {y: 2, z: 2}\nc:\n  <<: [*a, *b]\n  y: 3\n",
    ),
    (
        "merge list reversed order",
        "a: &a {x: 1, y: 1}\nb: &b {y: 2, z: 2}\nc:\n  <<: [*b, *a]\n",
    ),
    ("merge list of literal mappings", "c:\n  <<: [{x: 1}, {y: 2}]\n"),
    (
        "two merge keys in one mapping",
        "a: &a {x: 1}\nb: &b {x: 2, y: 2}\nc:\n  <<: *a\n  <<: *b\n",
    ),
    ("merge inside a flow mapping", "a: &a {x: 1}\nb: {<<: *a, y: 2}\n"),
    ("merge standing at the root", "base: &b {x: 1}\n<<: *b\ny: 2\n"),
    ("merge source defined at the merge site", "c:\n  <<: &s {x: 1}\nd: *s\n"),
    (
        "merge through an anchored mapping holding a merge",
        "a: &a {x: 1}\nb: &b\n  <<: *a\n  y: 2\nc:\n  <<: *b\n  z: 3\n",
    ),
    ("quoted merge key stands as no merge", "a: &a {x: 1}\nc:\n  '<<': *a\n"),
    ("merge key under the merge tag", "a: &a {x: 1}\nc:\n  !!merge <<: *a\n"),
    ("scalar merge source refuses", "c:\n  <<: 5\n"),
    ("null merge source refuses", "c:\n  <<: ~\n"),
    # Scalar tags.
    ("str tag reads a date spelling as text", "a: !!str 2026-10-14\n"),
    ("int tag reads a hex spelling", "a: !!int 0x1F\n"),
    ("int tag reads a binary spelling", "a: !!int 0b101\n"),
    ("int tag reads the 0o octal spelling", "a: !!int 0o17\n"),
    ("int tag reads a leading zero octal spelling", "a: !!int 017\n"),
    ("int tag reads a sexagesimal spelling", "a: !!int 1:30\n"),
    ("int tag reads a sexagesimal part past fifty nine", "a: !!int 1:99\n"),
    ("int tag reads an underscored decimal spelling", "a: !!int 1_000\n"),
    ("int tag refuses an invalid spelling", "a: !!int 08\n"),
    ("float tag reads an exponent spelling", "a: !!float 1e2\n"),
    ("float tag reads the infinity word", "a: !!float Infinity\n"),
    ("float tag reads a sexagesimal spelling", "a: !!float 1:30.5\n"),
    ("float tag reads a leading zero decimal as decimal", "a: !!float 017\n"),
    ("bool tag refuses a single letter", "a: !!bool y\n"),
    ("null tag discards the scalar text", "a: !!null hello\n"),
    ("non-specific tag on a scalar", "a: ! 5\n"),
    ("tagged empty scalar under str", "a: !!str\n"),
    # Collection tags.
    ("seq tag on a flow sequence", "a: !!seq [1, 2]\n"),
    ("map tag on a flow mapping", "a: !!map {k: 1}\n"),
    ("seq tag on a block sequence", "a: !!seq\n  - x\n  - y\n"),
    ("str tag on a collection refuses", "a: !!str [1]\n"),
    ("seq tag on a scalar refuses", "a: !!seq hello\n"),
    ("non-specific tag on a collection", "a: ! {k: 1}\n"),
    # Properties together and the block borders.
    ("tag in front of anchor on a scalar", "a: !!str &x 5\nb: *x\n"),
    ("anchor in front of a block head aliased later", "a: &x |\n  hi\nb: *x\n"),
    ("tag in front of a block head", "a: !!str |\n  hi\n"),
    ("int tag on a block scalar", "a: !!int |\n  42\n"),
    ("tagged block under strip chomping", "a: !!str |-\n  hi\n"),
    ("anchored block at the root", "&x |\n  hi\n"),
    ("tab inside an anchored block", "a: &x |\n  x\ty\n"),
    ("verbatim spelling of a standard tag", "a: !<tag:yaml.org,2002:str> 5\n"),
    # Expansion.
    ("document at the expansion bound reads", expansion_document(4000)),
    ("document past the expansion bound refuses", expansion_document(4001)),
]

# The string forms appended to yaml_strings, the document in
# each entry and its Python str and repr recorded from the
# runtime reading.
STRING_CASES = [
    "a: &s {x: 1}\nb:\n  <<: *s\n  y: 2\n",
    "a: &x [1, 2]\nb: *x\n",
    "a: !!timestamp 2001-12-15T02:59:43.1Z\n",
    "!!str 2026-10-14\n",
]


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
        by_text = case.get("text") in TEXT_FLIPS
        by_name = case.get("name") in NAME_FLIPS
        if (by_text or by_name) and case.get("port_stated"):
            case["expect"] = recorded(case["text"])
            if "ok" not in case["expect"]:
                raise SystemExit(f"the flip of {case.get('name')!r} does not read")
            del case["port_stated"]
            flipped.add(index)
    if len(flipped) != len(TEXT_FLIPS) + len(NAME_FLIPS):
        raise SystemExit(
            f"the standing flip cases stand flipped already or missing: {len(flipped)}"
        )
    for index, case in enumerate(values):
        if index not in flipped and json.dumps(case, sort_keys=True) != standing[index]:
            raise SystemExit("a standing yaml_values case would move outside the flips")
    standing_names = {case.get("name") for case in values}
    for name, text in CORPUS:
        if name in standing_names:
            raise SystemExit(f"corpus name {name!r} already stands in the recordings")
        expect, port_stated = corpus_expect(name, text)
        case = {"name": name, "text": text, "expect": expect}
        if port_stated:
            case["port_stated"] = True
        values.append(case)

    workflows = sections["workflows"]
    for case in workflows:
        if case.get("name") == "anchor value":
            if not case.get("port_stated"):
                raise SystemExit("the workflows anchor case stands flipped already")
            job = {
                "job": None,
                "job_name": None,
                "has_if": False,
                "combinations": None,
            }
            jobs = []
            for job_id in ("j", "k"):
                entry = dict(job)
                entry["job"] = job_id
                entry["job_name"] = job_id
                jobs.append([job_id, entry])
            case["expect"] = {
                "ok": [
                    {
                        "workflow": "w",
                        "file": "a.yml",
                        "triggers": {"t": "str", "v": "push"},
                        "jobs": jobs,
                    }
                ]
            }
            del case["port_stated"]
            break
    else:
        raise SystemExit("the workflows anchor case stands missing")

    strings = sections["yaml_strings"]
    for text in STRING_CASES:
        value = yaml.safe_load(text)
        strings.append(
            {"text": text, "expect": {"str": str(value), "repr": repr(value)}}
        )

    census_after = {name: len(cases) for name, cases in sections.items()}
    for name, cases in sections.items():
        if name in ("yaml_values", "yaml_strings", "workflows"):
            continue
        if json.dumps(cases, sort_keys=True) != before[name]:
            raise SystemExit(f"section {name} would move outside the stated movement")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    sys.exit(main())
