#!/usr/bin/env python3
"""Regenerate the PORT7 spanning quoted scalar recordings.

Runs PyYAML 6.0.1, the runtime reader the port mirrors, over
the corpus the contract's PORT7-6 names, and writes the
answers into tests/conformance/port1_cases.json.

The standing recordings of the earlier slices are carried
through untouched. This slice moves no standing recording,
the spanning family standing unrecorded in the standing
batteries and the flow folding correction standing on spans
no standing recording carries, and the corpus cases below
are appended to the yaml_values and yaml_strings sections
by their families. The script asserts that on every run, so
a regeneration that would move a standing recording fails
here rather than in the suite.

Two families stand recorded as port stated divergences
under the amendment the build of this slice writes. The
reader takes its text with every break form normalized to
the one line feed at the door, so a line separator or a
paragraph separator standing in a source document folds as
the break it normalizes to, where the runtime reading
keeps the separator standing as a character in the value.
Those cases carry the port's answer in the port stated
form the earlier slices use, and the pins of this slice
prove the port side of the divergence.
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


# The corpus, by the families PORT7-6 names. Each entry is a
# name and a document, recorded the same way as the corpora
# of the earlier slices.
CORPUS = [
    # 1. Spans standing in block value positions.
    ("double quoted span as a block value", 'key: "first\n  second"\n'),
    ("single quoted span as a block value", "key: 'first\n  second'\n"),
    ("double quoted span over three lines", 'key: "one\n  two\n  three"\n'),
    ("span with a blank line folds to one break", 'key: "first\n\n  second"\n'),
    ("span with two blank lines folds to two breaks", 'key: "first\n\n\n  second"\n'),
    ("span in a nested mapping value", 'outer:\n  key: "first\n    second"\n'),
    ("span as a sequence entry", '- "one\n  two"\n- three\n'),
    ("span in a mapping standing in a sequence entry", '- name: "first\n    second"\n'),
    ("span as the whole document", '"first\n  second"\n'),
    ("single quoted span as the whole document", "'first\n  second'\n"),
    ("span as the value of an explicit pair", '? key\n: "first\n  second"\n'),
    ("anchored span binds its folded text", 'key: &a "first\n  second"\nother: *a\n'),
    ("tagged span reads under its tag", 'key: !!str "first\n  second"\n'),
    ("pair after a span stands", 'key: "first\n  second"\nnext: 1\n'),
    ("closing quote at the head of a line", 'key: "first\nsecond"\n'),
    ("key shaped line inside a span is content", 'key: "first\n  second: not a key"\n'),
    ("comment shaped line inside a span is content", 'key: "first\n  # not a comment\n  second"\n'),
    # 2. The folding at the edges of a span and on escapes.
    ("break at the head of a span folds to a space", 'key: "\n  first"\n'),
    ("breaks at the head of a span fold to one break", 'key: "\n\n  first"\n'),
    ("break at the tail of a span folds to a space", 'key: "first\n  "\n'),
    ("breaks at both edges of a span", 'key: "\n\n  first\n\n"\n'),
    ("span of one break reads as one space", 'key: "\n"\n'),
    ("span of two breaks reads as one break", 'key: "\n\n"\n'),
    ("backslash at the end of a line joins with nothing", 'key: "first\\\n  second"\n'),
    ("escaped break never folds", 'key: "a\\nb"\n'),
    ("escaped space at the edges of a span stands", '{"k": "\\u0020ab\\u0020"}\n'),
    ("escaped tab beside a break stands", 'key: "a\\t\n  b"\n'),
    ("backslash in a single quoted span is a letter", "key: 'first\\\n  second'\n"),
    ("doubled quote across a span", "key: 'it''s\n  fine'\n"),
    ("carriage return span folds as a break", 'key: "first\r\n  second"\r\n'),
    ("tab at the head of a crossed line folds away", 'key: "first\n\tsecond"\n'),
    # 3. Spans standing in flow positions.
    ("double quoted span in a flow mapping", '{"k": "a\n  b"}\n'),
    ("single quoted span in a flow sequence", "- 'a\n  b'\n- c\n"),
    ("span in a flow value in block position", 'key: ["first\n  second"]\n'),
    ("break run at the head of a flow span", '{"k": "\n\n  first\n\n"}\n'),
    ("escaped break in a flow span never folds", '{"k": "a\\nb"}\n'),
    # 4. Spans standing as keys.
    ("spanning double quoted explicit key", '? "first\n  second"\n: value\n'),
    ("spanning single quoted explicit key", "? 'first\n  second'\n: value\n"),
    ("tagged spanning explicit key", '? !!str "first\n  second"\n: value\n'),
    ("anchored spanning explicit key binds", '? &a "first\n  second"\n: value\nother: *a\n'),
    ("spanning compact key refuses", '"first\n  second": value\n'),
    ("spanning flow key refuses", '{"first\n  second": 1}\n'),
    ("spanning single quoted flow key refuses", "{'first\n  second': 1}\n"),
    # 5. The borders the slice keeps.
    ("unterminated double quoted span refuses", 'key: "first\n  second\n'),
    ("unterminated single quoted span refuses", "key: 'first\n  second\n"),
    ("word after the closing quote of a span refuses", 'key: "first\n  second" junk\n'),
    ("comment after the closing quote of a span stands", 'key: "first\n  second" # note\n'),
    ("marker at the head of a crossed line refuses", 'a: "x\n---\ny"\n'),
    ("marker at the head of a crossed line in a single quoted span refuses", "a: 'x\n---\ny'\n"),
    ("end marker at the head of a crossed line refuses", 'a: "x\n...\ny"\n'),
    ("marker at the head of a crossed line in flow refuses", '{"k": "x\n---\ny"}\n'),
    ("marker after an escaped break refuses", 'a: "x\\\n---\ny"\n'),
    ("indented marker line inside a span is content", 'a: "x\n  ---\n  y"\n'),
    ("marker shaped word inside a span is content", 'a: "x\n---x\ny"\n'),
    ("marker on the opening line of a span is content", 'a: "---\ny"\n'),
    # 6. The standings the build probes added.
    ("blank before a backslash break stands", 'key: "first \\\n  second"\n'),
    ("two backslash breaks join twice", 'key: "a\\\n  b\\\n  c"\n'),
    ("spanning key node on the following lines", '?\n  "first\n  second"\n: value\n'),
    ("spanning single quoted key node on the following lines", "?\n  'first\n  second'\n: value\n"),
    ("spanning explicit value on the following lines", '? key\n:\n  "first\n  second"\n'),
    ("spanning key on a sequence entry line refuses", '- "first\n  second": v\n'),
    ("comment only line inside a span is content", 'key: "first\n# comment\n  second"\n'),
    ("span in a flow sequence in a flow mapping", '{"k": ["a\n  b"]}\n'),
    ("directive shaped line inside a span is content", 'a: "x\n%YAML 1.2\ny"\n'),
    ("directive shaped line inside a single quoted span is content", "a: 'x\n%YAML 1.2\ny'\n"),
]

# The port stated divergences of the amendment. Each entry is
# a name, a document, and the answer the port gives, stated
# in the tagged form. The runtime answers for these documents
# keep the separator standing as a character, and the port
# folds it as the break it normalizes to at the door.
PORT_STATED = [
    (
        "line separator inside a span folds as a break",
        'key: "a\u2028b"\n',
        {"t": "obj", "v": [[{"t": "str", "v": "key"}, {"t": "str", "v": "a b"}]]},
    ),
    (
        "paragraph separator inside a span folds as a break",
        'key: "a\u2029b"\n',
        {"t": "obj", "v": [[{"t": "str", "v": "key"}, {"t": "str", "v": "a b"}]]},
    ),
    (
        "line separator in a flow span folds as a break",
        '{"k": "a\u2028b"}\n',
        {"t": "obj", "v": [[{"t": "str", "v": "k"}, {"t": "str", "v": "a b"}]]},
    ),
]

# Root text spellings for the strings section, each a name
# and a document whose root value is a quoted scalar.
STRINGS = [
    ("root double quoted span", '"first\n  second"\n'),
    ("root single quoted span", "'first\n  second'\n"),
    ("root span with a blank line", '"first\n\n  second"\n'),
    ("root span of one break", '"\n"\n'),
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
    for name, text, expect in PORT_STATED:
        case = {"name": name, "text": text, "expect": {"ok": expect}, "port_stated": True}
        if name in standing_by_name:
            if json.dumps(standing_by_name[name], sort_keys=True) != json.dumps(
                case, sort_keys=True
            ):
                raise SystemExit(f"standing case {name!r} would move on a re-run")
            continue
        values.append(case)

    strings = sections["yaml_strings"]
    standing_texts = {case.get("text") for case in strings}
    for name, text in STRINGS:
        if text in standing_texts:
            continue
        value = yaml.safe_load(text)
        if not isinstance(value, str):
            raise SystemExit(f"strings case {name!r} does not read as text")
        case = {"text": text, "expect": {"str": value, "repr": repr(value)}}
        strings.append(case)

    census_after = {name: len(cases) for name, cases in sections.items()}
    for name, cases in sections.items():
        if name in ("yaml_values", "yaml_strings"):
            continue
        if json.dumps(cases, sort_keys=True) != before[name]:
            raise SystemExit(f"section {name} would move outside the stated movement")
    standing_values = json.loads(before["yaml_values"])
    for index, case in enumerate(standing_values):
        if json.dumps(values[index], sort_keys=True) != json.dumps(case, sort_keys=True):
            raise SystemExit("a standing yaml_values case would move")
    standing_strings = json.loads(before["yaml_strings"])
    for index, case in enumerate(standing_strings):
        if json.dumps(strings[index], sort_keys=True) != json.dumps(case, sort_keys=True):
            raise SystemExit("a standing yaml_strings case would move")
    RECORDINGS.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print("census before", census_before)
    print("census after", census_after)


if __name__ == "__main__":
    main()
