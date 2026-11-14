//! The pins of Rust port slice 3, the block scalar slice.
//! Every name PORT3-1 to PORT3-9 carries stands here as a
//! test against the reader, each one able to fail on the
//! reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};
use deadgate::yamlvalue::{YamlKey, YamlValue};

fn text(value: &str) -> YamlValue {
    YamlValue::Text(value.to_string())
}

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

fn want(value: YamlValue) -> Option<String> {
    Some(python_repr(&value))
}

fn mapping(pairs: Vec<(&str, YamlValue)>) -> YamlValue {
    YamlValue::Mapping(
        pairs
            .into_iter()
            .map(|(key, value)| (YamlKey::Text(key.to_string()), value))
            .collect(),
    )
}

// PORT3-1, the header grammar.

#[test]
fn test_a_block_header_with_a_comment_after_a_space_reads() {
    assert_eq!(
        reads("a: | # c\n  x\n"),
        want(mapping(vec![("a", text("x\n"))]))
    );
}

#[test]
fn test_a_block_header_comment_without_a_space_in_front_of_its_hash_is_refused() {
    assert_eq!(reads("a: |# c\n  x\n"), None);
}

#[test]
fn test_a_word_standing_after_the_block_header_is_refused() {
    assert_eq!(reads("a: | x\n  y\n"), None);
}

#[test]
fn test_a_block_header_with_two_chomping_marks_is_refused() {
    assert_eq!(reads("a: |+-\n  x\n"), None);
}

#[test]
fn test_a_block_header_with_two_indicator_digits_is_refused() {
    assert_eq!(reads("a: |12\n  x\n"), None);
}

#[test]
fn test_a_block_header_with_an_indicator_of_zero_is_refused() {
    assert_eq!(reads("a: |0\n  x\n"), None);
}

#[test]
fn test_a_block_header_written_with_two_digits_is_refused() {
    assert_eq!(reads("|12\n  x\n"), None);
}

#[test]
fn test_a_block_header_with_a_space_between_the_mark_and_its_indicator_is_refused() {
    assert_eq!(reads("a: | -\n  x\n"), None);
}

#[test]
fn test_a_block_header_with_an_indicator_written_as_a_letter_is_refused() {
    assert_eq!(reads("a: |a\n  x\n"), None);
}

#[test]
fn test_trailing_spaces_on_the_block_header_line_read() {
    assert_eq!(
        reads("a: |  \n  x\n"),
        want(mapping(vec![("a", text("x\n"))]))
    );
}

// PORT3-2, explicit indentation.

#[test]
fn test_a_first_content_line_short_of_the_indicator_width_is_refused() {
    assert_eq!(reads("a: |4\n  x\n"), None);
}

#[test]
fn test_content_past_the_indicator_width_keeps_its_excess_spaces() {
    assert_eq!(
        reads("a: |2\n    x\n"),
        want(mapping(vec![("a", text("  x\n"))]))
    );
}

#[test]
fn test_a_leading_spaces_only_line_past_the_indicator_width_carries_its_excess() {
    assert_eq!(
        reads("a: |+2\n     \n  x\n"),
        want(mapping(vec![("a", text("   \nx\n"))]))
    );
}

#[test]
fn test_a_block_scalar_under_an_explicit_indicator_reads_empty_before_a_sibling() {
    assert_eq!(
        reads("a: |2\nb: x\n"),
        want(mapping(vec![("a", text("")), ("b", text("x"))]))
    );
}

// PORT3-3, auto detection and spaces only lines.

#[test]
fn test_a_first_content_line_below_the_leading_blank_floor_is_refused() {
    assert_eq!(reads("a: |\n   \n  x\n"), None);
}

#[test]
fn test_a_later_line_below_the_block_indent_ends_the_scalar() {
    assert_eq!(
        reads("a: |\n  x\nb: 2\n"),
        want(mapping(vec![("a", text("x\n")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_spaces_only_line_past_the_block_indent_carries_its_excess_spaces() {
    assert_eq!(
        reads("a: |\n  x\n    \n  y\n"),
        want(mapping(vec![("a", text("x\n  \ny\n"))]))
    );
}

#[test]
fn test_a_leading_spaces_only_line_at_or_under_the_first_content_indent_contributes_its_break_alone(
) {
    assert_eq!(
        reads("a: |\n \n  x\n"),
        want(mapping(vec![("a", text("\nx\n"))]))
    );
}

// PORT3-4, assembly.

#[test]
fn test_a_break_beside_a_more_indented_line_is_never_folded() {
    assert_eq!(
        reads("a: >\n  one\n    ind\n  two\n"),
        want(mapping(vec![("a", text("one\n  ind\ntwo\n"))]))
    );
}

#[test]
fn test_a_blank_run_between_folded_lines_gives_one_break_per_blank_line() {
    assert_eq!(
        reads("a: >\n  one\n\n\n  two\n"),
        want(mapping(vec![("a", text("one\n\ntwo\n"))]))
    );
}

#[test]
fn test_the_folding_space_is_added_after_trailing_spaces() {
    assert_eq!(
        reads("a: >\n  one  \n  two\n"),
        want(mapping(vec![("a", text("one   two\n"))]))
    );
}

#[test]
fn test_a_more_indented_run_keeps_its_internal_breaks() {
    assert_eq!(
        reads("a: >\n  one\n    a\n    b\n  two\n"),
        want(mapping(vec![("a", text("one\n  a\n  b\ntwo\n"))]))
    );
}

#[test]
fn test_a_folded_line_opening_with_a_tab_keeps_the_breaks_around_it() {
    assert_eq!(
        reads("a: >2\n  first\n  \tsecond\n  third\n"),
        want(mapping(vec![("a", text("first\n\tsecond\nthird\n"))]))
    );
}

// PORT3-5, chomping.

#[test]
fn test_an_empty_block_scalar_is_the_empty_string_under_every_chomping() {
    for head in ["a: |", "a: |-", "a: |+"] {
        assert_eq!(
            reads(&format!("{head}\n")),
            want(mapping(vec![("a", text(""))])),
            "head {head}"
        );
    }
}

#[test]
fn test_a_blank_only_block_value_reads_empty_under_clip_and_strip_and_one_break_per_blank_line_under_keep(
) {
    assert_eq!(
        reads("a: |\n\n\nb: 1\n"),
        want(mapping(vec![("a", text("")), ("b", YamlValue::Int(1))]))
    );
    assert_eq!(
        reads("a: |-\n\n\nb: 1\n"),
        want(mapping(vec![("a", text("")), ("b", YamlValue::Int(1))]))
    );
    assert_eq!(
        reads("a: |+\n\n\nb: 1\n"),
        want(mapping(vec![("a", text("\n\n")), ("b", YamlValue::Int(1))]))
    );
}

#[test]
fn test_keep_keeps_no_break_the_source_does_not_hold() {
    assert_eq!(
        reads("a: |+\n  one"),
        want(mapping(vec![("a", text("one"))]))
    );
}

// PORT3-6, positions and the cursor standing.

#[test]
fn test_a_block_scalar_at_the_root_reads() {
    assert_eq!(reads("|\n  x\n"), want(text("x\n")));
}

#[test]
fn test_a_bare_dash_entry_carries_its_block_from_the_following_lines() {
    assert_eq!(
        reads("-\n  |\n    x\n- y\n"),
        want(YamlValue::List(vec![text("x\n"), text("y")]))
    );
}

#[test]
fn test_a_sibling_after_a_block_scalar_reads_as_it_reads_beside_a_plain_scalar() {
    assert_eq!(
        reads("a: |2\n  x\nb: 2\n"),
        want(mapping(vec![("a", text("x\n")), ("b", YamlValue::Int(2))]))
    );
}

// PORT3-7, the tab standing.

#[test]
fn test_a_tab_inside_block_content_stands_in_the_value() {
    assert_eq!(
        reads("a: |\n  one\ttwo\n"),
        want(mapping(vec![("a", text("one\ttwo\n"))]))
    );
}

#[test]
fn test_a_tab_in_block_indentation_is_refused() {
    assert_eq!(reads("a: |\n\tx\n"), None);
}

#[test]
fn test_a_tab_in_the_block_header_is_refused() {
    assert_eq!(reads("a: |\t\n  x\n"), None);
}

#[test]
fn test_a_tab_before_the_block_head_is_refused() {
    assert_eq!(reads("a:\t|\n  x\n"), None);
}

// PORT3-8, the apparatus standing.

#[test]
fn test_an_end_marker_line_indented_inside_a_block_is_content() {
    assert_eq!(
        reads("a: |\n  x\n  ...\n  y\n"),
        want(mapping(vec![("a", text("x\n...\ny\n"))]))
    );
}

#[test]
fn test_an_opener_line_indented_inside_a_block_is_content() {
    assert_eq!(
        reads("a: |\n  ---\n  y\n"),
        want(mapping(vec![("a", text("---\ny\n"))]))
    );
}

#[test]
fn test_a_percent_line_indented_inside_a_block_is_content() {
    assert_eq!(
        reads("a: |\n  %YAML\n  y\n"),
        want(mapping(vec![("a", text("%YAML\ny\n"))]))
    );
}

#[test]
fn test_an_end_marker_at_column_zero_closes_a_block_and_the_document() {
    assert_eq!(
        reads("a: |\n  x\n...\n"),
        want(mapping(vec![("a", text("x\n"))]))
    );
}

// PORT3-9, the value standing and the borders.

#[test]
fn test_an_empty_block_scalar_is_text_of_the_empty_string_and_never_null() {
    let value = parse_yaml("a: |\n").expect("the document reads");
    assert!(matches!(
        value.as_mapping().and_then(|pairs| pairs.first()),
        Some((YamlKey::Text(key), YamlValue::Text(empty))) if key == "a" && empty.is_empty()
    ));
}

#[test]
fn test_block_content_spelling_a_number_reads_as_text() {
    assert_eq!(
        reads("a: |\n  42\n"),
        want(mapping(vec![("a", text("42\n"))]))
    );
}

#[test]
fn test_block_content_spelling_a_boolean_or_a_date_or_a_null_word_reads_as_text() {
    for word in ["true", "2026-10-14", "null"] {
        assert_eq!(
            reads(&format!("a: |\n  {word}\n")),
            want(mapping(vec![("a", text(&format!("{word}\n")))])),
            "word {word}"
        );
    }
}

#[test]
fn test_a_block_head_in_flow_position_is_refused() {
    assert_eq!(reads("[| x]\n"), None);
    assert_eq!(reads("{a: |}\n"), None);
}

#[test]
fn test_a_block_scalar_as_a_key_is_refused() {
    // The refusal that stands after slice 5 is the block head
    // in compact key position, its colon standing on the head
    // line, where the runtime dies in its scanner. The block
    // scalar standing as an explicit key reads under PORT5-3
    // and is pinned there.
    assert_eq!(reads("|: 1\n"), None);
}

#[test]
fn test_block_content_carrying_a_line_or_paragraph_separator_reads_with_the_one_break() {
    assert_eq!(
        reads("a: |\n  one\u{2028}  two\n"),
        want(mapping(vec![("a", text("one\ntwo\n"))]))
    );
    assert_eq!(
        reads("a: |\n  one\u{2029}  two\n"),
        want(mapping(vec![("a", text("one\ntwo\n"))]))
    );
}

// The fix pass pins. The shapes the round's reviews found
// the first corpus missing, each one read against the
// runtime reading before the text of this file moved.

#[test]
fn test_a_block_holding_only_a_spaces_only_line_reads_empty_before_a_sibling() {
    assert_eq!(
        reads("a: |\n  \nb: 2\n"),
        want(mapping(vec![("a", text("")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_block_holding_only_a_spaces_only_line_under_keep_reads_its_break_alone() {
    assert_eq!(
        reads("a: |+\n  \nb: 2\n"),
        want(mapping(vec![("a", text("\n")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_block_holding_only_a_spaces_only_line_under_strip_reads_empty() {
    assert_eq!(
        reads("a: |-\n  \nb: 2\n"),
        want(mapping(vec![("a", text("")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_block_holding_only_a_wide_spaces_only_line_reads_empty() {
    assert_eq!(
        reads("a: |\n    \nb: 2\n"),
        want(mapping(vec![("a", text("")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_block_holding_only_a_spaces_only_line_in_a_sequence_reads_empty() {
    assert_eq!(
        reads("- |\n  \n- x\n"),
        want(YamlValue::List(vec![text(""), text("x")]))
    );
}

#[test]
fn test_a_spaces_only_line_carrying_a_tab_stands_as_content_before_a_sibling() {
    assert_eq!(
        reads("a: |\n  \t\nb: 2\n"),
        want(mapping(vec![("a", text("\t\n")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_spaces_only_line_carrying_a_tab_stands_as_content_in_a_sequence() {
    assert_eq!(
        reads("- |\n  \t\n- x\n"),
        want(YamlValue::List(vec![text("\t\n"), text("x")]))
    );
}

#[test]
fn test_a_wider_spaces_only_line_carrying_a_tab_stands_as_content_before_a_sibling() {
    assert_eq!(
        reads("a: |\n   \t\nb: 2\n"),
        want(mapping(vec![("a", text("\t\n")), ("b", YamlValue::Int(2))]))
    );
}

#[test]
fn test_a_tab_inside_block_content_reads_under_crlf_breaks() {
    assert_eq!(
        reads("a: |\r\n  x\ty\r\n"),
        want(mapping(vec![("a", text("x\ty\n"))]))
    );
}

#[test]
fn test_a_tab_inside_block_content_reads_under_cr_breaks() {
    assert_eq!(
        reads("a: |\r  x\ty\r"),
        want(mapping(vec![("a", text("x\ty\n"))]))
    );
}

#[test]
fn test_a_quote_inside_block_content_does_not_shelter_a_separator_tab() {
    assert_eq!(reads("x: |\n  'q\na:\t|\n  y\n"), None);
}

#[test]
fn test_a_leading_blank_floor_does_not_take_a_line_at_the_holding_indent_as_content() {
    assert_eq!(
        reads("a:\n  b: |\n \n  c: 1\n"),
        want(mapping(vec![(
            "a",
            mapping(vec![("b", text("")), ("c", YamlValue::Int(1))])
        )]))
    );
}

#[test]
fn test_a_folded_blank_run_before_a_more_indented_line_keeps_the_break_before_it() {
    assert_eq!(
        reads("a: >\n  one\n\n    ind\n"),
        want(mapping(vec![("a", text("one\n\n  ind\n"))]))
    );
}

#[test]
fn test_a_folded_blank_run_after_a_more_indented_line_keeps_the_break_before_it() {
    assert_eq!(
        reads("a: >\n  one\n    ind\n\n  two\n"),
        want(mapping(vec![("a", text("one\n  ind\n\ntwo\n"))]))
    );
}

#[test]
fn test_a_folded_blank_run_between_more_indented_lines_keeps_the_break_before_it() {
    assert_eq!(
        reads("a: >\n  one\n    mid\n\n    ind\n"),
        want(mapping(vec![("a", text("one\n  mid\n\n  ind\n"))]))
    );
}

#[test]
fn test_a_folded_blank_run_before_a_tab_first_line_keeps_the_break_before_it() {
    assert_eq!(
        reads("a: >\n  one\n\n  \ttwo\n"),
        want(mapping(vec![("a", text("one\n\n\ttwo\n"))]))
    );
}

#[test]
fn test_a_folded_blank_run_of_two_before_a_more_indented_line_keeps_every_break() {
    assert_eq!(
        reads("a: >\n  one\n\n\n    ind\n"),
        want(mapping(vec![("a", text("one\n\n\n  ind\n"))]))
    );
}
