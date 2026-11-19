//! Owner discharge diag for deadgate Round 16.

const CORPUS_A: &str = r##"# Round 15 scoping corpus S, the separator family. One document per
# line, escapes as the header of the Round 14 corpora states.
# quoted scalars, value position
k: "a\\u2028b"
k: "a\\u2029b"
k: 'a\\u2028b'
k: 'a\\u2029b'
k: "a\\u2028\\nb"
k: "a\\n\\u2028b"
k: "a\\u2028\\u2028b"
k: "a\\u2028\\n\\nb"
k: "\\u2028a"
k: "a\\u2028"
k: "a \\u2028 b"
k: "a\\u2028 \\nb"
k: "a\\u2028  b"
k: 'a\\u2028  b'
k: "a\\u2029\\n\\u2028b"
# quoted scalars, key position
? "a\\u2028b"\\n: v
"a\\u2028b": v
'a\\u2028b': v
# plain scalars
k: a\\u2028 b
k: a\\u2028b
k: a \\u2028b
k: a \\u2028 b
k: a\\u2028\\nb
k: a\\n  \\u2028b
a\\u2028 b: v
a\\u2028b: v
[a\\u2028 b, x]
[a\\u2028b, x]
a\\u2028 b
k: a\\u2029 b
k: a\\u2029b
# separators standing as separation and at the document frame
k:\\u2028v
a: 1\\u2028b: 2
k: v # c\\u2028k2: v2
%YAML 1.2\\u2028---\\na
a: b\\u2028
\\u2028a: b
---\\u2028a: b
[a,\\u2028b]
{a: b,\\u2028c: d}
[a\\u2028 b]
# block scalars
k: |\\n  a\\u2028b
k: |\\n  ab\\u2028
k: |\\n  ab\\u2028\\nj: w
k: |\\n  ab\\u2028\\n  cd
k: |\\n  \\u2028ab
k: |\\n  ab\\n  \\u2028\\n  cd
k: >\\n  a\\u2028b
k: >\\n  a\\u2028 b
k: >\\n  ab\\u2028\\n  cd\\n
k: >\\n  ab\\u2028
k: |-\\n  ab\\u2028
k: |+\\n  ab\\u2028\\n\\n
k: | # c\\u2028  ab
k: >\\n  ab\\n  c\\u2028d
k: |\\n  ab\\u2029
k: >\\n  a\\u2029 b
# next line controls
k: "a\\u0085b"
k: a\\u0085 b
k: |\\n  ab\\u0085
k: 'a\\u0085b'
"##;

const CORPUS_B: &str = r##"# Round 16 scoping corpus B, the separator family extension. One
# document per line, escapes as the header of the Round 14 corpora
# states, a section sign standing for one literal backslash with
# no further decoding on that line. Documents 06 and 07 are stream
# cells and stand read on the runtime side by the stream reading.
# whole content and the document frame
\\u2028
\\u2029
\\u2028\\na: b
a: b\\n\\u2028
k: v\\n\\u2028j: w
a: 1\\u2028---\\nb: 2
a: 1\\n...\\u2028b: 2
# anchors, aliases, tags, and the plain key colon
k: &a\\u2028b v
k: !!str\\u2028x
k: *a\\u2028b
k\\u2028: v
# colon and entry positions
k:\\u2028 v
k: \\u2028v
- \\u2028a
# quoted scalars, further standings
k: "ab\\ncd\\u2028"
k: 'ab\\ncd\\u2028'
k: "a\\u2028\\u2029b"
k: "a\\n\\u2028\\nb"
{"a\\u2028b": v}
? 'a\\u2028b'\\n: v
k: "\\u2028"
- "\\u2029"
# plain scalars, further standings
[a, b\\u2028 c]
[a\\u2028, b]
k: a\\u2028\\n  b
- a\\u2028 b
k: a \\u2028
a\\u2028b
{a\\u2028 b: v}
{a\\u2028b: v}
# block scalars, further standings
k: |2\\n   a\\u2028b
k: |\\n  ab\\u2028cd\\n  ef
k: |\\n  ab\\u2028  cd\\n  ef
k: >\\n  \\u2028\\n  cd
k: >-\\n  ab\\u2028\\n
k: |\\n   \\u2028\\n  cd
# directives, comments, and the escaped forms
%TAG !e! tag:ex,2000:app/\\u2028---\\na: b
# c\\u2028a: b
k: "a\\u000d\\u2028b"
k: "a§Lb"
k: "a§Pb"
"##;

const RUNTIME_A: &str = r##"01 | reads {'k': 'a\u2028b'}
02 | reads {'k': 'a\u2029b'}
03 | reads {'k': 'a\u2028b'}
04 | reads {'k': 'a\u2029b'}
05 | reads {'k': 'a\u2028\nb'}
06 | reads {'k': 'a\u2028b'}
07 | reads {'k': 'a\u2028\u2028b'}
08 | reads {'k': 'a\u2028\n\nb'}
09 | reads {'k': '\u2028a'}
10 | reads {'k': 'a\u2028'}
11 | reads {'k': 'a\u2028b'}
12 | reads {'k': 'a\u2028\nb'}
13 | reads {'k': 'a\u2028b'}
14 | reads {'k': 'a\u2028b'}
15 | reads {'k': 'a\u2029\n\u2028b'}
16 | reads {'a\u2028b': 'v'}
17 | refuses ScannerError
18 | refuses ScannerError
19 | reads {'k': 'a\u2028b'}
20 | refuses ScannerError
21 | refuses ScannerError
22 | reads {'k': 'a\u2028b'}
23 | refuses ScannerError
24 | refuses ScannerError
25 | refuses ScannerError
26 | refuses ScannerError
27 | reads ['a\u2028b', 'x']
28 | reads ['a\u2028b', 'x']
29 | reads 'a\u2028b'
30 | reads {'k': 'a\u2029b'}
31 | refuses ScannerError
32 | refuses ScannerError
33 | reads {'a': 1, 'b': 2}
34 | reads {'k': 'v', 'k2': 'v2'}
35 | reads 'a'
36 | reads {'a': 'b'}
37 | reads {'a': 'b'}
38 | reads {'a': 'b'}
39 | reads ['a', 'b']
40 | reads {'a': 'b', 'c': 'd'}
41 | reads ['a\u2028b']
42 | refuses ScannerError
43 | reads {'k': 'ab\u2028'}
44 | reads {'k': 'ab\u2028', 'j': 'w'}
45 | reads {'k': 'ab\u2028\ncd'}
46 | refuses ScannerError
47 | reads {'k': 'ab\n\u2028\ncd'}
48 | refuses ScannerError
49 | refuses ParserError
50 | reads {'k': 'ab\u2028\ncd\n'}
51 | reads {'k': 'ab\u2028'}
52 | reads {'k': 'ab'}
53 | reads {'k': 'ab\u2028\n\n'}
54 | reads {'k': 'ab'}
55 | refuses ScannerError
56 | reads {'k': 'ab\u2029'}
57 | refuses ParserError
58 | reads {'k': 'a b'}
59 | reads {'k': 'a b'}
60 | reads {'k': 'ab\n'}
61 | reads {'k': 'a b'}
"##;

const RUNTIME_B: &str = r##"01 | reads None
02 | reads None
03 | reads {'a': 'b'}
04 | reads {'a': 'b'}
05 | reads {'k': 'v', 'j': 'w'}
06 | refuses ComposerError
07 | refuses ParserError
08 | refuses ScannerError
09 | refuses ScannerError
10 | refuses ComposerError
11 | refuses ParserError
12 | reads {'k': 'v'}
13 | refuses ScannerError
14 | refuses ScannerError
15 | reads {'k': 'ab cd\u2028'}
16 | reads {'k': 'ab cd\u2028'}
17 | reads {'k': 'a\u2028\u2029b'}
18 | reads {'k': 'a\u2028\nb'}
19 | refuses ParserError
20 | reads {'a\u2028b': 'v'}
21 | reads {'k': '\u2028'}
22 | reads ['\u2029']
23 | reads ['a', 'b\u2028c']
24 | reads ['a', 'b']
25 | reads {'k': 'a\u2028\nb'}
26 | reads ['a\u2028b']
27 | reads {'k': 'a'}
28 | reads 'a\u2028b'
29 | refuses ParserError
30 | refuses ParserError
31 | refuses ScannerError
32 | refuses ScannerError
33 | reads {'k': 'ab\u2028cd\nef'}
34 | reads {'k': '\u2028\ncd'}
35 | reads {'k': 'ab'}
36 | refuses ParserError
37 | reads {'a': 'b'}
38 | reads {'k': 'a\u2028b'}
39 | reads {'k': 'a\u2028b'}
40 | reads {'k': 'a\u2029b'}
"##;

const PORT_A: &str = r##"01 | reads {'k': 'a b'}
02 | reads {'k': 'a b'}
03 | reads {'k': 'a b'}
04 | reads {'k': 'a b'}
05 | reads {'k': 'a\nb'}
06 | reads {'k': 'a\nb'}
07 | reads {'k': 'a\nb'}
08 | reads {'k': 'a\n\nb'}
09 | reads {'k': ' a'}
10 | reads {'k': 'a '}
11 | reads {'k': 'a b'}
12 | reads {'k': 'a\nb'}
13 | reads {'k': 'a b'}
14 | reads {'k': 'a b'}
15 | reads {'k': 'a\n\nb'}
16 | reads {'a b': 'v'}
17 | refuses
18 | refuses
19 | reads {'k': 'a b'}
20 | refuses
21 | refuses
22 | reads {'k': 'a b'}
23 | refuses
24 | refuses
25 | refuses
26 | refuses
27 | reads ['a b', 'x']
28 | reads ['a b', 'x']
29 | reads 'a b'
30 | reads {'k': 'a b'}
31 | refuses
32 | refuses
33 | reads {'a': 1, 'b': 2}
34 | reads {'k': 'v', 'k2': 'v2'}
35 | reads 'a'
36 | reads {'a': 'b'}
37 | reads {'a': 'b'}
38 | reads {'a': 'b'}
39 | reads ['a', 'b']
40 | reads {'a': 'b', 'c': 'd'}
41 | reads ['a b']
42 | refuses
43 | reads {'k': 'ab\n'}
44 | reads {'k': 'ab\n', 'j': 'w'}
45 | reads {'k': 'ab\n\ncd'}
46 | refuses
47 | reads {'k': 'ab\n\n\ncd'}
48 | refuses
49 | refuses
50 | reads {'k': 'ab\ncd\n'}
51 | reads {'k': 'ab\n'}
52 | reads {'k': 'ab'}
53 | reads {'k': 'ab\n\n\n'}
54 | reads {'k': 'ab'}
55 | refuses
56 | reads {'k': 'ab\n'}
57 | refuses
58 | reads {'k': 'a b'}
59 | reads {'k': 'a b'}
60 | reads {'k': 'ab\n'}
61 | reads {'k': 'a b'}
"##;

const PORT_B: &str = r##"01 | reads None
02 | reads None
03 | reads {'a': 'b'}
04 | reads {'a': 'b'}
05 | reads {'k': 'v', 'j': 'w'}
06 | refuses
07 | refuses
08 | refuses
09 | refuses
10 | refuses
11 | refuses
12 | reads {'k': 'v'}
13 | refuses
14 | refuses
15 | reads {'k': 'ab cd '}
16 | reads {'k': 'ab cd '}
17 | reads {'k': 'a\nb'}
18 | reads {'k': 'a\n\nb'}
19 | refuses
20 | reads {'a b': 'v'}
21 | reads {'k': ' '}
22 | reads [' ']
23 | reads ['a', 'b c']
24 | reads ['a', 'b']
25 | reads {'k': 'a\nb'}
26 | reads ['a b']
27 | reads {'k': 'a'}
28 | refuses
29 | reads {'a b': 'v'}
30 | reads {'a b': 'v'}
31 | refuses
32 | refuses
33 | reads {'k': 'ab\ncd\nef'}
34 | reads {'k': '\n\ncd'}
35 | reads {'k': 'ab'}
36 | refuses
37 | reads {'a': 'b'}
38 | reads {'k': 'a\nb'}
39 | reads {'k': 'a\u2028b'}
40 | reads {'k': 'a\u2029b'}
"##;

const MINI2: &str = r##"? a\\u2028 b\\n: v
k: a\\u2028 b\\n  c
? a\\u2028b\\n: v
"##;

use deadgate::yaml::{parse_yaml, python_repr};

/// Owner discharge diag for deadgate Round 16, probe sets A
/// to D, evidence only and never to be merged. Each corpus
/// document is decoded in the probers' escape form and read
/// through the port, and the reading is compared line by line
/// against the banked column the mode names. On the built
/// head AFTER stands true and the readings stand at the
/// runtime column, save the one root plain cell of PORT15-4
/// at corpus B document 28, which refuses under the stated
/// limit while the runtime reads it. On the contract head
/// AFTER stands false and the readings stand at the banked
/// port column of the landed code.
const AFTER: bool = false;

fn decode(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1] == '\\' {
            let c = chars[i + 2];
            match c {
                't' => {
                    out.push('\t');
                    i += 3;
                }
                'n' => {
                    out.push('\n');
                    i += 3;
                }
                '\\' => {
                    out.push('\\');
                    i += 3;
                }
                'u' => {
                    let hex: String = chars[i + 3..i + 7].iter().collect();
                    let value = u32::from_str_radix(&hex, 16).expect("hex");
                    out.push(char::from_u32(value).expect("codepoint"));
                    i += 7;
                }
                _ => panic!("bad escape {c}"),
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn documents(corpus: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in corpus.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains('\u{00a7}') {
            out.push(line.replace('\u{00a7}', "\\"));
        } else {
            out.push(decode(line));
        }
    }
    out
}

fn column(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            let body = line.split_once(" | ").expect("the column line stands").1;
            if body.starts_with("refuses") {
                "refuses".to_string()
            } else {
                body.to_string()
            }
        })
        .collect()
}

fn reading(doc: &str) -> String {
    match parse_yaml(doc) {
        Some(value) => format!("reads {}", python_repr(&value))
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029"),
        None => "refuses".to_string(),
    }
}

fn expected(corpus: usize, index: usize) -> String {
    let runtime = column(if corpus == 0 { RUNTIME_A } else { RUNTIME_B });
    let port = column(if corpus == 0 { PORT_A } else { PORT_B });
    if AFTER {
        if corpus == 1 && index == 28 {
            return "refuses".to_string();
        }
        runtime[index - 1].clone()
    } else {
        port[index - 1].clone()
    }
}

fn check(corpus: usize, range: &[usize]) {
    let docs = documents(if corpus == 0 { CORPUS_A } else { CORPUS_B });
    for &index in range {
        let got = reading(&docs[index - 1]);
        let want = expected(corpus, index);
        assert_eq!(
            got,
            want,
            "corpus {} document {} reads as the column reads",
            if corpus == 0 { "A" } else { "B" },
            index
        );
    }
}

fn seq(from: usize, to: usize) -> Vec<usize> {
    (from..=to).collect()
}

#[test]
fn test_set_a_the_quoted_grid() {
    check(0, &seq(1, 18));
    let mut b = seq(15, 22);
    b.extend(seq(38, 40));
    check(1, &b);
}

#[test]
fn test_set_b_the_plain_and_flow_grid() {
    let mut a = seq(19, 31);
    a.push(41);
    check(0, &a);
    check(1, &seq(23, 30));
    let docs = documents(MINI2);
    let want: Vec<String> = if AFTER {
        vec![
            "reads {'a\\u2028b': 'v'}".to_string(),
            "reads {'k': 'a\\u2028b c'}".to_string(),
            "refuses".to_string(),
        ]
    } else {
        vec![
            "refuses".to_string(),
            "reads {'k': 'a b c'}".to_string(),
            "refuses".to_string(),
        ]
    };
    for (i, doc) in docs.iter().enumerate() {
        assert_eq!(
            reading(doc),
            want[i],
            "the ad hoc plain border document {} reads as its column reads",
            i + 1
        );
    }
}

#[test]
fn test_set_c_the_block_grid() {
    check(0, &seq(42, 57));
    check(1, &seq(31, 36));
}

#[test]
fn test_set_d_the_boundary() {
    let mut a = seq(32, 40);
    a.extend(seq(58, 61));
    check(0, &a);
    let mut b = seq(1, 14);
    b.push(37);
    check(1, &b);
}
