//! Pins for slice PORT24, the anchored explicit key pair
//! slice in a flow sequence. An explicit key pair whose key
//! stands empty through an anchor refuses on the reader
//! where a break stands in the pair and misreads where none
//! stands, and the fences and borders around the shape
//! stand as PORT24 states them.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT24-1, the break refusal at the core.
#[test]
fn test_port24_an_anchored_empty_explicit_key_across_a_break_refuses_in_a_flow_sequence() {
    assert_eq!(reads("[? &a\n: 1]"), None);
    assert_eq!(reads("[? &a\u{2028}: 1]"), None);
    assert_eq!(reads("[? &a\u{2029}: 1]"), None);
    assert_eq!(reads("[? &a\r: 1]"), None);
    assert_eq!(reads("[? &a\n\n: 1]"), None);
    assert_eq!(reads("[? &a \n: 1]"), None);
    assert_eq!(reads("[? &a\n : 1]"), None);
    assert_eq!(reads("[? &a # c\n: 1]"), None);
}

// PORT24-2, the no break misread.
#[test]
fn test_port24_an_anchored_empty_explicit_key_with_no_break_reads_the_mark_and_anchor_as_a_plain_key(
) {
    assert_eq!(reads("[? &a : 1]"), Some("[{'? &a': 1}]".to_string()));
    assert_eq!(reads("[? &a: 1]"), Some("[{'? &a': 1}]".to_string()));
}

// PORT24-3, the seats and the values under the break.
#[test]
fn test_port24_the_anchored_empty_explicit_pair_reads_at_its_seats_and_values() {
    assert_eq!(reads("[? &a\n:]"), None);
    assert_eq!(reads("[? &a\n:, x]"), None);
    assert_eq!(reads("[? &a\n: 'v']"), None);
    assert_eq!(reads("[? &a\n: [1, 2]]"), None);
    assert_eq!(reads("[? &a\n: {b: 1}]"), None);
    assert_eq!(reads("[? &a\n: ~]"), None);
    assert_eq!(reads("[x, ? &a\n: 1]"), None);
    assert_eq!(reads("[? &a\n: 1, x]"), None);
    assert_eq!(reads("[[? &a\n: 1]]"), None);
    assert_eq!(reads("{k: [? &a\n: 1]}"), None);
}

// PORT24-4, the fences around the shape.
#[test]
fn test_port24_the_fences_around_the_anchored_empty_explicit_key_in_a_sequence_stand() {
    assert_eq!(reads("[? &a]"), Some("['? &a']".to_string()));
    assert_eq!(reads("[? &a\n]"), Some("['? &a']".to_string()));
    assert_eq!(reads("[? &a, x]"), Some("['? &a', 'x']".to_string()));
    assert_eq!(reads("[? &a\n, x]"), Some("['? &a', 'x']".to_string()));
}

// PORT24-5, the property and key borders of the shape.
#[test]
fn test_port24_the_property_and_key_borders_of_the_anchored_empty_explicit_sequence_pair_stand() {
    assert_eq!(reads("[? !!str\n: 1]"), None);
    assert_eq!(reads("[? &a !!str\n: 1]"), None);
    assert_eq!(reads("[? !!str &a\n: 1]"), None);
    assert_eq!(reads("[? !!str : 1]"), Some("[{'? !!str': 1}]".to_string()));
    assert_eq!(reads("[? !!str: 1]"), Some("[{'? !!str': 1}]".to_string()));
    assert_eq!(reads("[? !\n: 1]"), None);
    assert_eq!(reads("[? !!int\n: 1]"), None);
    assert_eq!(reads("[? &a k\n: 1]"), None);
    assert_eq!(reads("[? &a 'k'\n: 1]"), None);
    assert_eq!(reads("[? &a k: 1]"), Some("[{'? &a k': 1}]".to_string()));
}

// PORT24-6, the duplicate and neighbour standings.
#[test]
fn test_port24_the_duplicate_and_neighbour_standings_of_the_anchored_empty_explicit_pair_stand() {
    assert_eq!(reads("[? &a\n: 1, ? &a\n: 2]"), None);
    assert_eq!(
        reads("[? &a: 1, ? &a: 2]"),
        Some("[{'? &a': 1}, {'? &a': 2}]".to_string())
    );
    assert_eq!(reads("[? &a\n: 1, ? &b\n: 2]"), None);
    assert_eq!(reads("[&a\n: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a : 1]"), Some("[{None: 1}]".to_string()));
}
