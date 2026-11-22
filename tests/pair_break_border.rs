//! Pins for slice PORT22, the break border pair slice.
//! An anchored empty entry standing as a pair key in a
//! flow sequence, its anchor ending at a break, reads as
//! the pair on the reader against the runtime refusing
//! the document, and the borders around the pair stand
//! as PORT22-6 states them.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT22-1, the pair itself at its two break spellings.
#[test]
fn test_port22_an_anchored_empty_pair_key_with_a_break_behind_its_anchor_reads() {
    assert_eq!(reads("[&a\u{2028}: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n: 1]"), Some("[{None: 1}]".to_string()));
}

// PORT22-2, the wider break spellings behind the anchor.
#[test]
fn test_port22_a_paragraph_separator_and_a_carriage_return_behind_the_anchor_read_as_the_pair() {
    assert_eq!(reads("[&a\u{2029}: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\r: 1]"), Some("[{None: 1}]".to_string()));
}

// PORT22-3, the separation shapes behind the anchor.
#[test]
fn test_port22_the_separation_shapes_behind_the_anchor_read_as_the_pair() {
    assert_eq!(reads("[&a # c\n: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n : 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n\n: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n# c\n: 1]"), Some("[{None: 1}]".to_string()));
}

// PORT22-4, the property width of the border.
#[test]
fn test_port22_a_tag_ending_at_a_break_reads_as_the_pair() {
    assert_eq!(reads("[!!str\n: 1]"), Some("[{'': 1}]".to_string()));
    assert_eq!(reads("[!!str\u{2028}: 1]"), Some("[{'': 1}]".to_string()));
    assert_eq!(reads("[&a !!str\n: 1]"), Some("[{'': 1}]".to_string()));
    assert_eq!(reads("[!!str &a\n: 1]"), Some("[{'': 1}]".to_string()));
}

// PORT22-5, the seats the pair stands at.
#[test]
fn test_port22_the_pair_reads_at_its_seats() {
    assert_eq!(reads("[x, &a\n: 1]"), Some("['x', {None: 1}]".to_string()));
    assert_eq!(reads("[[&a\n: 1]]"), Some("[[{None: 1}]]".to_string()));
    assert_eq!(
        reads("{k: [&a\n: 1]}"),
        Some("{'k': [{None: 1}]}".to_string())
    );
    assert_eq!(
        reads("[&a\u{2028}: 1, b: 2]"),
        Some("[{None: 1}, {'b': 2}]".to_string())
    );
    assert_eq!(reads("[&a\n:]"), Some("[{None: None}]".to_string()));
}

// PORT22-6, the borders that part the pair from its family.
#[test]
fn test_port22_the_borders_around_the_pair_stand() {
    assert_eq!(reads("[&a : 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n, x]"), Some("[None, 'x']".to_string()));
    assert_eq!(reads("[&a\n]"), Some("[None]".to_string()));
    assert_eq!(reads("{&a\n: 1}"), None);
    assert_eq!(reads("{&a\u{2028}: 1}"), None);
    assert_eq!(reads("{[&a\n: 1]: v}"), None);
    assert_eq!(reads("[&a\t: 1]"), None);
}
