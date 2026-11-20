//! The pins of Rust port slice 15, the separator family.
//! Every name PORT15-2 and PORT15-3 and PORT15-4 and
//! PORT15-5 and PORT15-6 carry stands here as a test
//! against the reader, each one able to fail on the
//! reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT15-2, the quoted folding by kind, one folding for
// every position a quoted scalar stands.

#[test]
fn test_a_line_separator_standing_alone_in_a_span_reads_as_itself() {
    assert_eq!(
        reads("key: \"a\u{2028}b\"\n"),
        Some("{'key': 'a\\u2028b'}".to_string())
    );
}

#[test]
fn test_a_paragraph_separator_standing_alone_in_a_span_reads_as_itself() {
    assert_eq!(
        reads("key: \"a\u{2029}b\"\n"),
        Some("{'key': 'a\\u2029b'}".to_string())
    );
}

#[test]
fn test_a_span_holding_only_a_separator_reads_as_the_separator() {
    assert_eq!(
        reads("key: \"\u{2028}\"\n"),
        Some("{'key': '\\u2028'}".to_string())
    );
}

#[test]
fn test_a_run_of_a_separator_and_a_line_feed_keeps_every_break() {
    assert_eq!(
        reads("key: \"x\u{2028}\ny\"\n"),
        Some("{'key': 'x\\u2028\\ny'}".to_string())
    );
}

#[test]
fn test_a_run_opening_with_a_line_feed_drops_that_break_and_keeps_the_separator_behind_it() {
    assert_eq!(
        reads("key: \"x\n\u{2028}y\"\n"),
        Some("{'key': 'x\\u2028y'}".to_string())
    );
}

#[test]
fn test_the_mixed_run_of_the_two_separators_keeps_both_standing() {
    assert_eq!(
        reads("key: \"a\u{2028}\u{2029}b\"\n"),
        Some("{'key': 'a\\u2028\\u2029b'}".to_string())
    );
}

#[test]
fn test_a_separator_at_the_head_of_a_span_and_at_its_tail_stands() {
    assert_eq!(
        reads("key: \"\u{2028}a\u{2029}\"\n"),
        Some("{'key': '\\u2028a\\u2029'}".to_string())
    );
}

#[test]
fn test_a_single_quoted_span_carrying_a_separator_reads_as_the_double_quoted_span_reads() {
    assert_eq!(
        reads("key: 'a\u{2028}b'\n"),
        Some("{'key': 'a\\u2028b'}".to_string())
    );
}

#[test]
fn test_an_explicit_quoted_key_carrying_a_separator_keeps_the_separator_standing() {
    assert_eq!(
        reads("? \"a\u{2028}b\"\n: 1\n"),
        Some("{'a\\u2028b': 1}".to_string())
    );
}

// PORT15-3, a separator standing as content inside block
// scalar content.

#[test]
fn test_a_separator_ending_a_literal_content_line_stands_as_content() {
    assert_eq!(
        reads("a: |\n  one\u{2028}\n  two\n"),
        Some("{'a': 'one\\u2028\\ntwo\\n'}".to_string())
    );
}

#[test]
fn test_a_separator_ending_a_folded_content_line_gives_a_break_rather_than_a_space() {
    assert_eq!(
        reads("a: >\n  one\u{2028}\n  two\n"),
        Some("{'a': 'one\\u2028\\ntwo\\n'}".to_string())
    );
}

#[test]
fn test_a_line_holding_only_a_separator_inside_a_literal_block_reads_as_a_content_line() {
    assert_eq!(
        reads("a: |\n  one\n  \u{2028}\n  two\n"),
        Some("{'a': 'one\\n\\u2028\\ntwo\\n'}".to_string())
    );
}

#[test]
fn test_a_line_holding_only_a_separator_inside_a_folded_block_keeps_the_character_standing() {
    assert_eq!(
        reads("a: >\n  one\n  \u{2028}\n  two\n"),
        Some("{'a': 'one\\u2028\\ntwo\\n'}".to_string())
    );
}

#[test]
fn test_content_standing_behind_a_separator_at_the_block_indent_joins_with_nothing_between() {
    assert_eq!(
        reads("a: |\n  one\u{2028}  two\n"),
        Some("{'a': 'one\\u2028two\\n'}".to_string())
    );
}

#[test]
fn test_a_trailing_separator_under_keep_chomping_stands_as_content() {
    assert_eq!(
        reads("a: |+\n  one\u{2029}"),
        Some("{'a': 'one\\u2029'}".to_string())
    );
}

#[test]
fn test_content_standing_behind_a_separator_at_the_block_indent_in_folded_style_joins_with_nothing_between(
) {
    assert_eq!(
        reads("a: >\n  one\u{2028}  two\n"),
        Some("{'a': 'one\\u2028two\\n'}".to_string())
    );
}

// PORT15-4, a separator standing as content in the plain
// readings, and the one limit the slice keeps.

#[test]
fn test_a_separator_with_a_space_behind_it_stands_in_a_block_plain_value() {
    assert_eq!(
        reads("a: x\u{2028} y\n"),
        Some("{'a': 'x\\u2028y'}".to_string())
    );
}

#[test]
fn test_a_separator_at_the_end_of_a_plain_line_stands_with_the_lines_own_break_behind_it() {
    assert_eq!(
        reads("a: x\u{2028}\n y\n"),
        Some("{'a': 'x\\u2028\\ny'}".to_string())
    );
}

#[test]
fn test_a_separator_inside_a_plain_scalar_stands_while_the_physical_break_folds_to_a_space() {
    assert_eq!(
        reads("a: x\u{2028} y\n z\n"),
        Some("{'a': 'x\\u2028y z'}".to_string())
    );
}

#[test]
fn test_an_explicit_plain_key_carrying_a_separator_reads_with_the_character_standing() {
    assert_eq!(
        reads("? x\u{2028} y\n: 1\n"),
        Some("{'x\\u2028y': 1}".to_string())
    );
}

#[test]
fn test_a_separator_with_a_space_behind_it_stands_in_a_sequence_entry() {
    assert_eq!(reads("- x\u{2029} y\n"), Some("['x\\u2029y']".to_string()));
}

#[test]
fn test_a_root_plain_scalar_carrying_a_separator_with_no_space_behind_it_still_refuses() {
    // The one limit PORT15-4 keeps. The runtime reading
    // reads the document as one scalar with the character
    // standing, and the port refuses it, and this pin
    // holds the border of the limit exactly where the
    // obligation states it.
    assert_eq!(reads("x\u{2028}y\n"), None);
}

// PORT15-5, the separators in the flow readings.

#[test]
fn test_a_separator_inside_a_flow_plain_scalar_stands_as_content() {
    assert_eq!(reads("[x\u{2028}y]\n"), Some("['x\\u2028y']".to_string()));
    assert_eq!(reads("[x\u{2028} y]\n"), Some("['x\\u2028y']".to_string()));
}

#[test]
fn test_a_separator_inside_a_flow_plain_key_is_refused() {
    assert_eq!(reads("{x\u{2028} y: 1}\n"), None);
    assert_eq!(reads("{x\u{2028}y: 1}\n"), None);
}

#[test]
fn test_a_separator_inside_a_flow_quoted_span_reads_as_it_reads_in_block_position() {
    assert_eq!(
        reads("{\"k\": \"a\u{2028}b\"}\n"),
        Some("{'k': 'a\\u2028b'}".to_string())
    );
}

// PORT15-6, the borders that do not move.

#[test]
fn test_a_separator_alone_on_a_line_between_two_entries_acts_as_the_break_it_is() {
    assert_eq!(
        reads("a: 1\n\u{2028}\nb: 2\n"),
        Some("{'a': 1, 'b': 2}".to_string())
    );
}

#[test]
fn test_the_next_line_character_inside_a_span_folds_to_a_space_as_it_does_today() {
    assert_eq!(
        reads("key: \"a\u{85}b\"\n"),
        Some("{'key': 'a b'}".to_string())
    );
}

#[test]
fn test_a_separator_spelled_by_an_escape_stands_as_content_as_it_does_today() {
    assert_eq!(
        reads("key: \"a\\Lb\"\n"),
        Some("{'key': 'a\\u2028b'}".to_string())
    );
}

#[test]
fn test_a_separator_standing_as_the_whole_document_reads_as_the_empty_document() {
    assert_eq!(reads("\u{2028}"), Some("None".to_string()));
}

// The pins of the repair of Rust port slice 15, standing
// as an addition at the end of tests/separators.rs behind
// the pins of the slice itself. Each pin names the note
// that carries it in its section comment.

// The repair beside PORT15-4, the explicit key gathering
// stopping at a line carrying a pair of its own.

#[test]
fn test_a_pair_line_behind_a_separator_ended_explicit_key_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} b: 2\n: 1\n"), None);
    assert_eq!(reads("? a\u{2028} b:\n: 1\n"), None);
}

#[test]
fn test_a_quoted_pair_line_behind_a_separator_ended_explicit_key_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} \"b\": 2\n: 1\n"), None);
}

#[test]
fn test_a_flow_shaped_pair_line_behind_a_separator_ended_explicit_key_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} [b]: 2\n: 1\n"), None);
}

#[test]
fn test_a_pair_line_at_a_deeper_indent_behind_an_empty_segment_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028}\n  b: 2\n: 1\n"), None);
    assert_eq!(reads("? a\u{2028}\n\n  b: 2\n: 1\n"), None);
}

#[test]
fn test_a_pair_line_with_no_colon_line_behind_it_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} b: 2"), None);
    assert_eq!(reads("? a\u{2028} b: 2\n"), None);
}

#[test]
fn test_a_pair_line_behind_a_paragraph_separator_ended_explicit_key_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2029} b: 2\n: 1\n"), None);
}

#[test]
fn test_a_pair_line_standing_behind_a_gathered_chunk_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} b\n c: 2\n: 1\n"), None);
}

#[test]
fn test_a_dash_line_behind_a_separator_ended_explicit_key_folds_as_text() {
    assert_eq!(
        reads("? a\u{2028} - b\n: 1\n"),
        Some("{'a\\u2028- b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028}\n  - b\n: 1\n"),
        Some("{'a\\u2028\\n- b': 1}".to_string())
    );
    assert_eq!(
        reads("? a\u{2028} -b\n: 1\n"),
        Some("{'a\\u2028-b': 1}".to_string())
    );
}

#[test]
fn test_a_colon_with_no_space_behind_it_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} b:2\n: 1\n"),
        Some("{'a\\u2028b:2': 1}".to_string())
    );
}

// The repair beside PORT15-5, the flow plain key refusal
// reading the token as it stands scanned.

#[test]
fn test_a_flow_plain_key_ending_at_a_separator_is_refused() {
    assert_eq!(reads("{a\u{2028}: 1}\n"), None);
}

#[test]
fn test_a_flow_plain_key_ending_at_a_separator_with_a_space_in_front_of_it_is_refused() {
    assert_eq!(reads("{a \u{2028}: 1}\n"), None);
}

#[test]
fn test_a_flow_plain_key_ending_at_a_separator_with_a_space_behind_it_is_refused() {
    assert_eq!(reads("{a\u{2028} : 1}\n"), None);
}

#[test]
fn test_a_flow_plain_key_ending_at_a_paragraph_separator_is_refused() {
    assert_eq!(reads("{a\u{2029}: 1}\n"), None);
    assert_eq!(reads("{a\u{2029} : 1}\n"), None);
}

#[test]
fn test_a_flow_plain_key_ending_at_a_separator_inside_a_nested_mapping_is_refused() {
    assert_eq!(reads("[{a\u{2028}: 1}]\n"), None);
    assert_eq!(reads("{k: {a\u{2028}: 1}}\n"), None);
}

// The repair beside PORT15-5, the flow anchor and tag
// token end sets taking the separators as separation.

#[test]
fn test_a_flow_anchor_ending_at_a_separator_stands_as_separation() {
    assert_eq!(reads("[&a\u{2028} b]\n"), Some("['b']".to_string()));
}

#[test]
fn test_a_flow_tag_ending_at_a_separator_stands_as_separation() {
    assert_eq!(reads("[!!str\u{2028} a]\n"), Some("['a']".to_string()));
}

#[test]
fn test_a_flow_anchor_standing_against_a_separator_reads_the_entry_behind_it() {
    assert_eq!(reads("[&a\u{2028}b]\n"), Some("['b']".to_string()));
    assert_eq!(reads("[&a\u{2029}b]\n"), Some("['b']".to_string()));
}

#[test]
fn test_a_flow_anchor_ending_at_a_separator_before_a_comma_stands_as_separation() {
    assert_eq!(reads("[&a\u{2028}, b]\n"), Some("[None, 'b']".to_string()));
}

#[test]
fn test_a_flow_anchor_ending_at_a_paragraph_separator_stands_as_separation() {
    assert_eq!(reads("[&a\u{2029} b]\n"), Some("['b']".to_string()));
}

#[test]
fn test_a_flow_tag_ending_at_a_paragraph_separator_stands_as_separation() {
    assert_eq!(reads("[!!str\u{2029} a]\n"), Some("['a']".to_string()));
}

// The repair beside PORT15-3, the block corner and the
// trailing standings standing pinned as proof completions.

#[test]
fn test_a_block_whose_only_content_line_is_a_separator_reads_empty_under_clip_chomping() {
    assert_eq!(reads("a: |\n  \u{2028}\n"), Some("{'a': ''}".to_string()));
    assert_eq!(reads("a: |\n  \u{2029}\n"), Some("{'a': ''}".to_string()));
    assert_eq!(reads("a: >\n  \u{2028}\n"), Some("{'a': ''}".to_string()));
}

#[test]
fn test_a_block_whose_only_content_line_is_a_separator_keeps_the_separator_under_keep_chomping() {
    assert_eq!(
        reads("a: |+\n  \u{2028}\n"),
        Some("{'a': '\\u2028\\n'}".to_string())
    );
    assert_eq!(
        reads("a: |+\n  \u{2029}\n"),
        Some("{'a': '\\u2029\\n'}".to_string())
    );
    assert_eq!(
        reads("a: >+\n  \u{2028}\n"),
        Some("{'a': '\\u2028\\n'}".to_string())
    );
}

#[test]
fn test_a_block_whose_only_content_line_is_a_separator_reads_empty_under_strip_chomping() {
    assert_eq!(reads("a: |-\n  \u{2028}\n"), Some("{'a': ''}".to_string()));
    assert_eq!(reads("a: |-\n  \u{2029}\n"), Some("{'a': ''}".to_string()));
    assert_eq!(reads("a: >-\n  \u{2029}\n"), Some("{'a': ''}".to_string()));
}

#[test]
fn test_a_trailing_separator_under_clip_chomping_stands_as_content() {
    assert_eq!(
        reads("a: |\n  one\u{2028}\n"),
        Some("{'a': 'one\\u2028'}".to_string())
    );
    assert_eq!(
        reads("a: |\n  one\u{2029}\n"),
        Some("{'a': 'one\\u2029'}".to_string())
    );
    assert_eq!(
        reads("a: >\n  one\u{2028}\n"),
        Some("{'a': 'one\\u2028'}".to_string())
    );
    assert_eq!(
        reads("a: >\n  one\u{2029}\n"),
        Some("{'a': 'one\\u2029'}".to_string())
    );
}

#[test]
fn test_a_trailing_line_separator_under_keep_chomping_stands_with_the_break_behind_it() {
    assert_eq!(
        reads("a: |+\n  one\u{2028}\n"),
        Some("{'a': 'one\\u2028\\n'}".to_string())
    );
}

#[test]
fn test_a_trailing_separator_under_strip_chomping_reads_as_nothing() {
    assert_eq!(
        reads("a: |-\n  one\u{2028}\n"),
        Some("{'a': 'one'}".to_string())
    );
    assert_eq!(
        reads("a: |-\n  one\u{2029}\n"),
        Some("{'a': 'one'}".to_string())
    );
}
