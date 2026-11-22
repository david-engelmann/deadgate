//! The pins of Rust port slice 17, the colon fold slice.
//! Every name PORT17-1 and PORT17-2 carry stands here as
//! a test against the reader, each one able to fail on
//! the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT17-1, the gathering stopping at a gathered line
// carrying a mapping value colon standing in dress.

#[test]
fn test_an_opening_flow_mapping_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} ? {b: 2}\n: 1\n"), None);
}

#[test]
fn test_an_opening_quoted_pair_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} ? \"b\" c: 2\n: 1\n"), None);
}

#[test]
fn test_an_opening_quoted_colon_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} ? \"b: c\"\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_quoted_pair_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} \"b\" c: 2\n: 1\n"), None);
}

#[test]
fn test_a_quoted_colon_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} \"b: c\"\n: 1\n"), None);
}

#[test]
fn test_a_single_quoted_pair_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} 'b' c: 2\n: 1\n"), None);
}

#[test]
fn test_a_single_quoted_colon_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} 'b: c'\n: 1\n"), None);
}

#[test]
fn test_an_opening_flow_mapping_behind_a_paragraph_separator_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2029} ? {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_on_a_second_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} foo\u{2028} {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_on_a_gathered_line_in_a_nested_mapping_stops_the_gathering_and_refuses() {
    assert_eq!(reads("outer:\n  ? a\u{2028}\n   {b: 2}\n  : 1\n"), None);
}

#[test]
fn test_a_flow_mapping_behind_a_blank_run_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028}\n\n {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_two_pair_flow_mapping_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} {b: 2, c: 3}\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_whose_colon_stands_at_the_gathered_lines_end_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} {b:\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_carrying_a_quoted_colon_on_a_gathered_line_stops_the_gathering_and_refuses()
{
    assert_eq!(reads("? a\u{2028} {\"b: c\": 2}\n: 1\n"), None);
}

#[test]
fn test_a_property_in_front_of_a_flow_mapping_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} &p {b: 2}\n: 1\n"), None);
    assert_eq!(reads("? a\u{2028} !!str {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_quoted_span_whose_colon_and_space_stand_at_its_end_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} \"b: \"\n: 1\n"), None);
}

#[test]
fn test_a_sequence_entry_mapping_on_a_gathered_line_stops_the_gathering_and_refuses() {
    assert_eq!(reads("? a\u{2028} - {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_plain_pair_behind_a_flow_sequence_holding_a_quoted_entry_on_a_gathered_line_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} ['b'] c: 2\n: 1\n"), None);
}

#[test]
fn test_a_plain_pair_behind_a_flow_sequence_carrying_no_colon_on_a_gathered_line_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} [b, c] d: 2\n: 1\n"), None);
}

#[test]
fn test_a_plain_pair_behind_a_flow_mapping_carrying_no_colon_on_a_gathered_line_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} {b} c: 2\n: 1\n"), None);
}

#[test]
fn test_a_flow_mapping_behind_a_stray_closing_bracket_on_a_gathered_line_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} ] {b: 2}\n: 1\n"), None);
}

#[test]
fn test_a_quoted_colon_behind_a_stray_closing_bracket_on_a_gathered_line_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} ] \"b: c\"\n: 1\n"), None);
}

// PORT17-2, the borders of the fold standing unmoved.

#[test]
fn test_a_flow_sequence_carrying_no_colon_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} [b, c]\n: 1\n"),
        Some("{'a\\u2028[b, c]': 1}".to_string())
    );
}

#[test]
fn test_a_quoted_span_carrying_no_colon_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} \"b c\"\n: 1\n"),
        Some("{'a\\u2028\"b c\"': 1}".to_string())
    );
}

#[test]
fn test_a_flow_mapping_whose_colon_carries_no_space_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} {b:2}\n: 1\n"),
        Some("{'a\\u2028{b:2}': 1}".to_string())
    );
}

#[test]
fn test_a_quoted_span_whose_colon_stands_at_its_end_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} \"b:\"\n: 1\n"),
        Some("{'a\\u2028\"b:\"': 1}".to_string())
    );
}

#[test]
fn test_a_plain_pair_line_behind_a_separator_ended_explicit_key_still_stops_the_gathering_and_refuses(
) {
    assert_eq!(reads("? a\u{2028} b: 2\n: 1\n"), None);
}

#[test]
fn test_a_colon_behind_a_comment_mark_inside_a_double_quoted_span_folds_as_text_in_a_gathered_key()
{
    assert_eq!(
        reads("? a\u{2028} \"b # c: d\"\n: 1\n"),
        Some("{'a\\u2028\"b': 1}".to_string())
    );
}

#[test]
fn test_a_colon_behind_a_comment_mark_inside_a_single_quoted_span_folds_as_text_in_a_gathered_key()
{
    assert_eq!(
        reads("? a\u{2028} 'b # c: d'\n: 1\n"),
        Some("{\"a\\u2028'b\": 1}".to_string())
    );
}

#[test]
fn test_a_flow_mapping_behind_a_comment_mark_behind_a_property_folds_as_text_in_a_gathered_key() {
    assert_eq!(
        reads("? a\u{2028} &p # {b: 2}\n: 1\n"),
        Some("{'a\\u2028&p': 1}".to_string())
    );
}

// PORT18-1, the pair refusal at the two pair takings of
// the flow reading.

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_line_separator_refuses() {
    assert_eq!(reads("[a\u{2028}: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_paragraph_separator_refuses() {
    assert_eq!(reads("[a\u{2029}: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_line_feed_refuses() {
    assert_eq!(reads("[a\n: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_with_a_separator_inside_its_key_refuses() {
    assert_eq!(reads("[a\u{2028}b: c]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_with_a_line_feed_inside_its_key_refuses() {
    assert_eq!(reads("[a\nb: c]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_comment_ending_at_a_line_feed_refuses() {
    assert_eq!(reads("[a # c\n: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_comment_ending_at_a_line_separator_refuses() {
    assert_eq!(reads("[a # c\u{2028}: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_with_a_quoted_key_spanning_a_separator_refuses() {
    assert_eq!(reads("['a'\u{2028}: b]"), None);
    assert_eq!(reads("[\"a\"\u{2028}: b]"), None);
    assert_eq!(reads("[\"a\"\n: b]"), None);
    assert_eq!(reads("['a'\n: b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_with_a_break_inside_its_quoted_key_refuses() {
    assert_eq!(reads("['a\n': b]"), None);
    assert_eq!(reads("[\"a\n\": b]"), None);
    assert_eq!(reads("['a\u{2028}': b]"), None);
    assert_eq!(reads("[\"a\u{2028}\": b]"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_separator_standing_second_refuses() {
    assert_eq!(reads("[x, a\u{2028}: b]"), None);
    assert_eq!(reads("[[a\u{2028}: b]]"), None);
    assert_eq!(reads("{k: [a\u{2028}: b]}"), None);
}

#[test]
fn test_port18_a_flow_sequence_pair_spanning_a_separator_with_an_empty_value_refuses() {
    assert_eq!(reads("[a\u{2028}:]"), None);
}

#[test]
fn test_port18_a_flow_mapping_pair_spanning_a_line_feed_refuses() {
    assert_eq!(reads("{a\n: b}"), None);
}

#[test]
fn test_port18_a_flow_mapping_pair_with_a_line_feed_inside_its_key_refuses() {
    assert_eq!(reads("{a\nb: c}"), None);
}

#[test]
fn test_port18_a_flow_mapping_pair_spanning_a_comment_refuses() {
    assert_eq!(reads("{a # c\n: b}"), None);
    assert_eq!(reads("{a # c\u{2028}: b}"), None);
}

#[test]
fn test_port18_a_flow_mapping_pair_with_a_quoted_key_spanning_a_line_feed_refuses() {
    assert_eq!(reads("{\"a\"\n: b}"), None);
    assert_eq!(reads("{'a'\n: b}"), None);
}

#[test]
fn test_port18_a_flow_mapping_pair_with_a_collection_key_spanning_a_line_feed_refuses() {
    assert_eq!(reads("{[a]\n: b}"), None);
}

// PORT18-2, the borders of the crossing standing unmoved.

#[test]
fn test_port18_a_flow_sequence_pair_standing_whole_on_one_line_reads() {
    assert_eq!(reads("[a: b]"), Some("[{'a': 'b'}]".to_string()));
    assert_eq!(reads("[a : b]"), Some("[{'a': 'b'}]".to_string()));
}

#[test]
fn test_port18_a_flow_pair_with_a_break_behind_its_colon_reads() {
    assert_eq!(reads("[a:\nb]"), Some("[{'a': 'b'}]".to_string()));
    assert_eq!(reads("{a:\nb}"), Some("{'a': 'b'}".to_string()));
    assert_eq!(reads("[a: # c\nb]"), Some("[{'a': 'b'}]".to_string()));
}

#[test]
fn test_port18_a_plain_entry_carrying_a_separator_inside_itself_reads() {
    assert_eq!(reads("[a\u{2028}b]"), Some("['a\u{2028}b']".to_string()));
    assert_eq!(
        reads("{k: [a\u{2028}b]}"),
        Some("{'k': ['a\u{2028}b']}".to_string())
    );
    assert_eq!(reads("[a\u{2028}, b]"), Some("['a', 'b']".to_string()));
}

#[test]
fn test_port18_a_flow_sequence_pair_with_a_separator_inside_its_value_reads() {
    assert_eq!(
        reads("[a: b\u{2028}c]"),
        Some("[{'a': 'b\u{2028}c'}]".to_string())
    );
}

#[test]
fn test_port18_a_comment_standing_in_front_of_a_flow_mapping_key_reads() {
    assert_eq!(
        reads("{a: 1, # c\u{2028}b: 2}"),
        Some("{'a': 1, 'b': 2}".to_string())
    );
}

#[test]
fn test_port18_a_flow_mapping_pair_spanning_a_separator_still_refuses() {
    assert_eq!(reads("{a\u{2028}: b}"), None);
    assert_eq!(reads("{a\u{2028}b: c}"), None);
}
