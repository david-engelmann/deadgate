#!/usr/bin/env python3
"""Regenerate the PORT3 block scalar recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the block scalar corpus the contract's PORT3-10 names, and
writes the answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched except for the movement PORT3-10 states.
The two yaml_values block cases recorded as port stated
refusals flip to their recorded values, the workflows block
scalar case flips to a recorded workflow that reads whole,
the yaml_strings section gains the root literal and root
folded cases, the corpus cases below are appended to the
yaml_values section, and the separator cases stand as port
stated divergences, their answers taken from the runtime
reading of the normalized text, never as the raw answers
the runtime reading gives on the separator itself.

Nothing else in the recordings moves. The script asserts
that on every run, so a regeneration that would move a
standing recording outside the stated movement fails here
rather than in the suite.
"""

import json
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
RECORDINGS = ROOT / "tests" / "conformance" / "port1_cases.json"


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
    """The runtime answer for a document, as an expect field."""
    try:
        value = yaml.safe_load(text)
    except yaml.YAMLError as error:
        return {"err": type(error).__name__}
    return {"ok": tag(value)}


# The corpus, by the families PORT3-10 names. Each entry is a
# name and a document. The answers are recorded, never set
# down here.
CORPUS = [
    # 1. Style and chomping at a mapping value (A, O).
    ("block clip folded two lines", "a: >\n  one\n  two\n"),
    ("block strip literal", "a: |-\n  one\n  two\n"),
    ("block strip folded no final break", "a: >-\n  one\n  two"),
    ("block keep literal trailing blank", "a: |+\n  one\n\n"),
    ("block keep folded trailing blanks at end of file", "a: >+\n  one\n\n\n"),
    ("block clip no final break", "a: |\n  one\n  two"),
    ("block keep no final break", "a: |+\n  one"),
    ("block strip trailing blanks before a sibling", "a: |-\n  one\n\n\nb: 2\n"),
    ("block clip trailing blanks before a sibling", "a: |\n  one\n\n\nb: 2\n"),
    ("block keep trailing blanks before a sibling", "a: |+\n  one\n\n\nb: 2\n"),
    ("block folded clip before a sibling", "a: >\n  one\n  two\n\nb: 2\n"),
    # 2. Empty and blank first scalars (B, O06).
    ("block empty clip before a sibling", "a: |\nb: 1\n"),
    ("block empty strip at end of file", "a: |-\n"),
    ("block empty keep at end of file", "a: |+\n"),
    ("block blank only clip", "a: |\n\n\nb: 1\n"),
    ("block blank only keep", "a: |+\n\n\nb: 1\n"),
    ("block blank only strip", "a: |-\n\n\n"),
    ("block blanks before content clip", "a: |\n\n\n  x\n"),
    ("block blanks before content keep", "a: |+\n\n  x\n"),
    # 3. Explicit indicators (C, L, I04).
    ("block explicit digit in the middle", "a: |2\n  x\n"),
    ("block explicit digit one", "a: |1\n x\n"),
    ("block explicit digit nine", "a: |9\n         x\n"),
    ("block explicit chomping first", "a: |-2\n  x\n"),
    ("block explicit digit first strip", "a: |2-\n  x\n"),
    ("block explicit keep chomping first", "a: |+2\n  x\n"),
    ("block explicit keep digit first", "a: |2+\n  x\n\n"),
    ("block explicit content past the width", "a: |2\n    x\n"),
    ("block explicit first line short refuses", "a: |4\n  x\n"),
    ("block explicit reads empty before a sibling", "a: |2\nb: x\n"),
    ("block explicit at the root", "|2\n  x\n"),
    ("block explicit sequence entry", "- |2\n  x\n- y\n"),
    ("block explicit sequence entry mapping", "- run: |2\n    echo hi\n"),
    ("block explicit nested mapping value", "a:\n  b: |2\n    x\n  c: 1\n"),
    ("block explicit leading spaces only past the width", "a: |+2\n     \n  x\n"),
    # 4. Auto detection (J, B15, C19, D11, D17, I03).
    ("block auto later line below refuses", "a: |\n    x\n  y\n"),
    ("block auto sibling ends the scalar", "a: |\n  x\nb: 2\n"),
    ("block auto leading blank floor refuses", "a: |\n   \n  x\n"),
    ("block auto leading blank floor reads", "a: |\n \n  x\n"),
    ("block auto sequence sibling ends the scalar", "- |\n    x\n- y\n"),
    ("block auto later line past the indent", "a: |\n  x\n    y\n"),
    ("block auto nested sibling reads", "a:\n  b: >\n      ind\n  c: 2\n"),
    # 5. Folding mechanics (D, P01, J01 to J03).
    ("block folded plain run", "a: >\n  one\n  two\n  three\n"),
    ("block folded blank run of one", "a: >\n  one\n\n  two\n"),
    ("block folded blank run of two", "a: >\n  one\n\n\n  two\n"),
    ("block folded blank run of three", "a: >\n  one\n\n\n\n  two\n"),
    ("block folded more indented first line", "a: >\n    ind\n  one\n"),
    ("block folded more indented middle", "a: >\n  one\n    ind\n  two\n"),
    ("block folded more indented end", "a: >\n  one\n    ind\n"),
    ("block folded more indented run", "a: >\n  one\n    a\n    b\n  two\n"),
    ("block literal trailing spaces", "a: |\n  one  \n  two\n"),
    ("block folded trailing spaces", "a: >\n  one  \n  two\n"),
    ("block folded blank first line", "a: >\n\n    x\n    y\n"),
    ("block literal blank first line", "a: |\n\n    x\n    y\n"),
    # 6. Spaces only lines (K, L10, D20, I01, P06).
    ("block spaces only at the width", "a: |\n  x\n  \n  y\n"),
    ("block spaces only past the width literal", "a: |\n  x\n    \n  y\n"),
    ("block spaces only past the width folded", "a: >\n  x\n    \n  y\n"),
    ("block spaces only below the width", "a: |\n  x\n \n  y\n"),
    ("block spaces only carrying a tab", "a: |\n  x\n  \t\n  y\n"),
    # 7. Positions (E).
    ("block at the root literal", "|\n  x\n"),
    ("block at the root folded", ">\n  x\n  y\n"),
    ("block deep mapping value", "a:\n  b:\n    c: |\n      deep\n"),
    ("block dash line folded entry", "- >\n  x\n  y\n"),
    ("block bare dash entry", "-\n  |\n    x\n- y\n"),
    ("block sibling blocks", "a: |\n  one\nb: >\n  two\n"),
    ("block workflow run shape", "steps:\n  - run: |\n      echo hi\n      echo bye\n"),
    # 8. Header grammar (F, C14 to C18, I05).
    ("block header comment after a space", "a: | # c\n  x\n"),
    ("block header comment without a space refuses", "a: |# c\n  x\n"),
    ("block header trailing spaces read", "a: |  \n  x\n"),
    ("block header junk after refuses", "a: | x\n  y\n"),
    ("block header word after chomping refuses", "a: |- x\n  y\n"),
    ("block header separated indicator refuses", "a: | -\n  x\n"),
    ("block header two chomping marks refuse", "a: |+-\n  x\n"),
    ("block header doubled minus refuses", "a: |--\n  x\n"),
    ("block header indicator zero refuses", "a: |0\n  x\n"),
    ("block header two digits refuse", "a: |12\n  x\n"),
    ("block header letter indicator refuses", "a: |a\n  x\n"),
    # 9. Tabs (G, P03, P06).
    ("block tab inside a literal line", "a: |\n  one\ttwo\n"),
    ("block tab as first content character", "a: |\n  \tx\n"),
    ("block tab inside a folded line", "a: >\n  one\ttwo\n"),
    ("block tab in indentation refuses", "a: |\n\tx\n"),
    ("block tab opening a later line refuses", "a: |\n  x\n\ty\n"),
    ("block tab after the header mark refuses", "a: |\t\n  x\n"),
    ("block folded tab as first content character", "a: >\n  \tx\n"),
    # 10. Document apparatus (H, N).
    ("block end marker at column zero", "a: |\n  x\n...\n"),
    ("block end marker with a comment", "a: |\n  x\n... # end\n"),
    ("block end marker with junk refuses", "a: |\n  x\n... junk\n"),
    ("block opener at column zero refuses", "a: |\n  x\n---\nb: 1\n"),
    ("block end marker indented is content", "a: |\n  x\n  ...\n  y\n"),
    ("block opener indented is content", "a: |\n  ---\n  y\n"),
    ("block percent indented is content", "a: |\n  %YAML\n  y\n"),
    ("block percent at column zero refuses", "a: |\n  x\n%YAML 1.2\n"),
    # 11. The standing refusals that border the lift (E07 to E11, F08, F09).
    ("block head in a flow mapping refuses", "{a: |}\n"),
    ("block head after a flow entry refuses", "[1, |]\n"),
    ("block head behind an anchor refuses", "a: &x |\n  y\n"),
    ("block head behind a tag refuses", "a: !!str |\n  y\n"),
    ("block scalar as an explicit key refuses", "? |\n  x\n: 1\n"),
    # 12. Resolution standing (E13, E14).
    ("block content spelling a number", "a: |\n  42\n"),
    ("block content spelling a boolean", "a: |\n  true\n"),
    ("block content spelling a date", "a: |\n  2026-10-14\n"),
    ("block content spelling a null word", "a: |\n  null\n"),
]

# The border cases the port keeps refused while the runtime
# reading accepts them, the anchor and the tag and the
# explicit key families of PORT3-9. Their recordings stand
# as port stated refusals, in the standing form.
PORT_REFUSALS = {
    "block head behind an anchor refuses",
    "block head behind a tag refuses",
    "block scalar as an explicit key refuses",
}


def corpus_expect(name, text):
    if name in PORT_REFUSALS:
        return {"err": "port refusal"}, True
    return recorded(text), False


# The separator cases, standing as port stated divergences
# under the limit PORT3 states. The recorded answer is the
# runtime reading of the normalized text, in which the
# separator stands as the one break, as the port reads it.
SEPARATOR_CORPUS = [
    ("block line separator inside literal content", "a: |\n  one\u2028two\n"),
    ("block paragraph separator at the end under keep", "a: |+\n  one\u2029"),
    ("block line separator inside folded content", "a: >\n  one\u2028two\n"),
]


def normalized(text):
    return (
        text.replace("\r\n", "\n")
        .replace("\r", "\n")
        .replace("\u0085", "\n")
        .replace("\u2028", "\n")
        .replace("\u2029", "\n")
    )


def main():
    if yaml.__version__ != "6.0.1":
        raise SystemExit(f"PyYAML stands at {yaml.__version__}, the corpus wants 6.0.1")
    document = json.loads(RECORDINGS.read_text(encoding="utf-8"))
    sections = document["sections"]
    before = {name: json.dumps(cases, sort_keys=True) for name, cases in sections.items()}
    census_before = {name: len(cases) for name, cases in sections.items()}

    values = sections["yaml_values"]
    # The two standing block cases flip to their recorded
    # values, found by their text.
    flips = {"a: |\n  block\n", "a: >\n  folded\n"}
    found = set()
    for case in values:
        if case.get("text") in flips and case.get("port_stated"):
            case["expect"] = recorded(case["text"])
            del case["port_stated"]
            found.add(case["text"])
    if found != flips:
        raise SystemExit(f"the standing block cases stand flipped already or missing: {found!r}")
    standing_names = {case.get("name") for case in values}
    for name, text in CORPUS:
        if name in standing_names:
            raise SystemExit(f"corpus name {name!r} already stands in the recordings")
        expect, port_stated = corpus_expect(name, text)
        case = {"name": name, "text": text, "expect": expect}
        if port_stated:
            case["port_stated"] = True
        values.append(case)
    for name, text in SEPARATOR_CORPUS:
        if name in standing_names:
            raise SystemExit(f"corpus name {name!r} already stands in the recordings")
        values.append(
            {
                "name": name,
                "text": text,
                "expect": recorded(normalized(text)),
                "port_stated": True,
            }
        )

    workflows = sections["workflows"]
    for case in workflows:
        if case.get("name") == "block scalar":
            if not case.get("port_stated"):
                raise SystemExit("the workflows block case stands flipped already")
            case["expect"] = {
                "ok": [
                    {
                        "workflow": "w",
                        "file": "a.yml",
                        "triggers": {"t": "str", "v": "push"},
                        "jobs": [
                            [
                                "j",
                                {
                                    "job": "j",
                                    "job_name": "j",
                                    "has_if": False,
                                    "combinations": None,
                                },
                            ]
                        ],
                    }
                ]
            }
            del case["port_stated"]
            break
    else:
        raise SystemExit("the workflows block case stands missing")

    strings = sections["yaml_strings"]
    for text in ("|\n  block\n", ">\n  folded\n"):
        value = yaml.safe_load(text)
        strings.append(
            {"text": text, "expect": {"str": value, "repr": repr(value)}}
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
