//! Pins for slice PORT21, the tab crossing slice. A tab
//! inside a quoted span stands exempt at the door and
//! folds in a gathered key, a tab in the plain state
//! refuses at the door and again at the plain value arm
//! behind it, and the beside standings pass the door.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT21-1, the door exemption at the quoted value seats.
#[test]
fn test_port21_a_tab_inside_a_quoted_span_in_a_quoted_value_reads() {
    assert_eq!(
        reads("a: \"x\t# y\"\n"),
        Some("{'a': 'x\\t# y'}".to_string())
    );
    assert_eq!(reads("a: 'x\t# y'\n"), Some("{'a': 'x\\t# y'}".to_string()));
    assert_eq!(reads("a: \"b\tc\"\n"), Some("{'a': 'b\\tc'}".to_string()));
}

// PORT21-2, the door refusal borders.
#[test]
fn test_port21_a_tab_at_the_door_in_the_plain_state_and_behind_a_span_and_mid_word_refuses() {
    assert_eq!(reads("a: x\t# y\n"), None);
    assert_eq!(reads("a: \"x\"\t# y\n"), None);
    assert_eq!(reads("a: x\"y\t# z\"\n"), None);
    assert_eq!(reads("a:\tx\n"), None);
}

// PORT21-3, the gathered key twins.
#[test]
fn test_port21_a_tab_inside_a_span_on_a_gathered_key_line_folds_as_a_space() {
    assert_eq!(
        reads("? a\u{2028} \"b\t# c: d\"\n: 1\n"),
        Some("{'a\\u2028\"b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} 'b\t# c: d'\n: 1\n"),
        Some("{\"a\\u2028'b\": 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} \"b # c: d\"\n: 1\n"),
        Some("{'a\\u2028\"b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2029} \"b\t# c: d\"\n: 1\n"),
        Some("{'a\\u2029\"b': 1}".to_string())
    );
}

// PORT21-4, the mark free span on a gathered line.
#[test]
fn test_port21_a_tab_inside_a_mark_free_span_stands_in_the_folded_key() {
    assert_eq!(
        reads("? a\u{2028} \"b\tc\"\n: 1\n"),
        Some("{'a\\u2028\"b\\tc\"': 1}".to_string())
    );
}

// PORT21-5, the gathering running behind a cut span chunk.
#[test]
fn test_port21_the_gathering_runs_behind_a_cut_span_chunk() {
    assert_eq!(
        reads("? a\u{2028} \"b\t# c\" d\n: 1\n"),
        Some("{'a\\u2028\"b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} \"b\t# c\"\n d\n: 1\n"),
        Some("{'a\\u2028\"b d': 1}".to_string())
    );
}

// PORT21-6, property spellings in a gathered chunk.
#[test]
fn test_port21_a_property_spelling_in_a_gathered_chunk_folds_as_text() {
    assert_eq!(
        reads("? a\u{2028} &p \"b\t# c: d\"\n: 1\n"),
        Some("{'a\\u2028&p \"b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} !t \"b\t# c: d\"\n: 1\n"),
        Some("{'a\\u2028!t \"b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} &p \"b # c: d\"\n: 1\n"),
        Some("{'a\\u2028&p \"b': 1}".to_string())
    );
}

// PORT21-7, the pair behind a mark free span.
#[test]
fn test_port21_a_pair_behind_a_mark_free_span_stops_the_gathering() {
    assert_eq!(reads("? a\u{2028} &p \"b\tc\" e: 1\n: 2\n"), None);
}

// PORT21-8, the tab at the plain value seats.
#[test]
fn test_port21_a_tab_inside_a_span_at_the_plain_value_seats_refuses() {
    assert_eq!(reads("a: x \"b\t# c\"\n"), None);
    assert_eq!(reads("a: x\n \"b\tc\"\n"), None);
    assert_eq!(reads("a: x \"b\tc\"\n"), None);
}

// PORT21-9, the beside standings at the door.
#[test]
fn test_port21_a_tab_in_a_comment_and_in_block_content_passes_the_door() {
    assert_eq!(reads("a: x # c\td\n"), Some("{'a': 'x'}".to_string()));
    assert_eq!(
        reads("a: |\n x\n \ty\n"),
        Some("{'a': 'x\\n\\ty\\n'}".to_string())
    );
}
