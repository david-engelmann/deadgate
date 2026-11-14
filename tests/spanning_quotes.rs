//! The pins of Rust port slice 7, the spanning quoted scalar
//! slice. Every name PORT7-1 to PORT7-5 carries stands here
//! as a test against the reader, each one able to fail on the
//! reading it pins, beside the two pins of the slice's stated
//! limit on the separator characters, which the amendment in
//! the contract names. The two separator names PORT7-2
//! carries stand disposed as recording cases in the battery,
//! as the amendment states, and carry no test here.

use deadgate::yaml::{parse_yaml, python_repr};
use deadgate::yamlvalue::{YamlKey, YamlValue};

fn text(value: &str) -> YamlValue {
    YamlValue::Text(value.to_string())
}

fn int(value: i64) -> YamlValue {
    YamlValue::Int(value)
}

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

fn want(value: YamlValue) -> Option<String> {
    Some(python_repr(&value))
}

fn mapping(pairs: Vec<(YamlKey, YamlValue)>) -> YamlValue {
    YamlValue::Mapping(pairs)
}

fn text_key(key: &str) -> YamlKey {
    YamlKey::Text(key.to_string())
}

fn list(items: Vec<YamlValue>) -> YamlValue {
    YamlValue::List(items)
}

// PORT7-1, spans standing in block value positions.

#[test]
fn test_a_double_quoted_scalar_spanning_lines_reads_as_a_block_value() {
    assert_eq!(
        reads("key: \"first\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
    assert_eq!(
        reads("outer:\n  key: \"first\n    second\"\n"),
        want(mapping(vec![(
            text_key("outer"),
            mapping(vec![(text_key("key"), text("first second"))]),
        )]))
    );
    assert_eq!(
        reads("key: \"first\nsecond\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
}

#[test]
fn test_a_single_quoted_scalar_spanning_lines_reads_as_a_block_value() {
    assert_eq!(
        reads("key: 'first\n  second'\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
    assert_eq!(
        reads("key: 'it''s\n  fine'\n"),
        want(mapping(vec![(text_key("key"), text("it's fine"))]))
    );
}

#[test]
fn test_a_spanning_scalar_reads_as_a_sequence_entry() {
    assert_eq!(
        reads("- \"one\n  two\"\n- three\n"),
        want(list(vec![text("one two"), text("three")]))
    );
    assert_eq!(
        reads("- name: \"first\n    second\"\n"),
        want(list(vec![mapping(vec![(
            text_key("name"),
            text("first second"),
        )])]))
    );
}

#[test]
fn test_a_spanning_scalar_reads_as_the_whole_document() {
    assert_eq!(reads("\"first\n  second\"\n"), want(text("first second")));
    assert_eq!(reads("'first\n  second'\n"), want(text("first second")));
    assert_eq!(
        reads("key: [\"first\n  second\"]\n"),
        want(mapping(vec![(
            text_key("key"),
            list(vec![text("first second")]),
        )]))
    );
}

#[test]
fn test_an_anchored_spanning_scalar_binds_its_folded_text() {
    assert_eq!(
        reads("key: &a \"first\n  second\"\nother: *a\n"),
        want(mapping(vec![
            (text_key("key"), text("first second")),
            (text_key("other"), text("first second")),
        ]))
    );
    assert_eq!(
        reads("key: !!str \"first\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
}

#[test]
fn test_the_pair_after_a_spanning_scalar_stands() {
    assert_eq!(
        reads("key: \"first\n  second\"\nnext: 1\n"),
        want(mapping(vec![
            (text_key("key"), text("first second")),
            (text_key("next"), int(1)),
        ]))
    );
}

// PORT7-2, the one folding.

#[test]
fn test_a_single_break_folds_to_a_space() {
    assert_eq!(
        reads("key: \"first\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
    assert_eq!(
        reads("key: \"first\n\tsecond\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
}

#[test]
fn test_a_run_of_breaks_folds_to_one_fewer_break() {
    assert_eq!(
        reads("key: \"first\n\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first\nsecond"))]))
    );
    assert_eq!(
        reads("key: \"first\n\n\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first\n\nsecond"),)]))
    );
    assert_eq!(
        reads("key: \"\n\n\"\n"),
        want(mapping(vec![(text_key("key"), text("\n"))]))
    );
}

#[test]
fn test_a_break_at_the_head_of_a_span_folds_as_a_break_between_lines() {
    assert_eq!(
        reads("key: \"\n  first\"\n"),
        want(mapping(vec![(text_key("key"), text(" first"))]))
    );
    assert_eq!(
        reads("key: \"\n\n  first\"\n"),
        want(mapping(vec![(text_key("key"), text("\nfirst"))]))
    );
    assert_eq!(
        reads("key: \"first\n  \"\n"),
        want(mapping(vec![(text_key("key"), text("first "))]))
    );
    assert_eq!(
        reads("key: \"\n\n  first\n\n\"\n"),
        want(mapping(vec![(text_key("key"), text("\nfirst\n"))]))
    );
    assert_eq!(
        reads("key: \"\n\"\n"),
        want(mapping(vec![(text_key("key"), text(" "))]))
    );
}

#[test]
fn test_a_backslash_at_the_end_of_a_line_joins_with_nothing_standing() {
    assert_eq!(
        reads("key: \"first\\\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("firstsecond"))]))
    );
    assert_eq!(
        reads("key: \"a\\\n  b\\\n  c\"\n"),
        want(mapping(vec![(text_key("key"), text("abc"))]))
    );
    assert_eq!(
        reads("key: \"first \\\n  second\"\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
}

#[test]
fn test_an_escaped_line_break_never_folds() {
    assert_eq!(
        reads("key: \"a\\nb\"\n"),
        want(mapping(vec![(text_key("key"), text("a\nb"))]))
    );
    assert_eq!(
        reads("{\"k\": \"a\\nb\"}\n"),
        want(mapping(vec![(text_key("k"), text("a\nb"))]))
    );
}

#[test]
fn test_an_escaped_space_at_the_edge_of_a_span_stands() {
    assert_eq!(
        reads("{\"k\": \"\\u0020ab\\u0020\"}\n"),
        want(mapping(vec![(text_key("k"), text(" ab "))]))
    );
    assert_eq!(
        reads("key: \"a\\t\n  b\"\n"),
        want(mapping(vec![(text_key("key"), text("a\t b"))]))
    );
}

// PORT7-3, the flow reading conformed.

#[test]
fn test_a_flow_span_and_a_block_span_of_one_text_read_alike() {
    assert_eq!(
        reads("{\"k\": \"a\n  b\"}\n"),
        want(mapping(vec![(text_key("k"), text("a b"))]))
    );
    assert_eq!(
        reads("key: \"a\n  b\"\n"),
        want(mapping(vec![(text_key("key"), text("a b"))]))
    );
    assert_eq!(
        reads("- 'a\n  b'\n- c\n"),
        want(list(vec![text("a b"), text("c")]))
    );
    assert_eq!(
        reads("{\"k\": [\"a\n  b\"]}\n"),
        want(mapping(vec![(text_key("k"), list(vec![text("a b")]),)]))
    );
}

#[test]
fn test_a_break_run_at_the_head_of_a_flow_span_folds_as_the_runtime_folds_it() {
    assert_eq!(
        reads("{\"k\": \"\n\n  first\n\n\"}\n"),
        want(mapping(vec![(text_key("k"), text("\nfirst\n"))]))
    );
    assert_eq!(
        reads("{\"k\": \"first\\\n  second\"}\n"),
        want(mapping(vec![(text_key("k"), text("firstsecond"))]))
    );
}

// PORT7-4, spans standing as keys.

#[test]
fn test_a_spanning_quoted_explicit_key_reads() {
    assert_eq!(
        reads("? \"first\n  second\"\n: value\n"),
        want(mapping(vec![(text_key("first second"), text("value"))]))
    );
    assert_eq!(
        reads("? 'first\n  second'\n: value\n"),
        want(mapping(vec![(text_key("first second"), text("value"))]))
    );
    assert_eq!(
        reads("?\n  \"first\n  second\"\n: value\n"),
        want(mapping(vec![(text_key("first second"), text("value"))]))
    );
    assert_eq!(
        reads("{? \"first\n  second\": value}\n"),
        want(mapping(vec![(text_key("first second"), text("value"))]))
    );
    assert_eq!(
        reads("? !!str \"first\n  second\"\n: value\n"),
        want(mapping(vec![(text_key("first second"), text("value"))]))
    );
    assert_eq!(
        reads("? &a \"first\n  second\"\n: value\nother: *a\n"),
        want(mapping(vec![
            (text_key("first second"), text("value")),
            (text_key("other"), text("first second")),
        ]))
    );
}

#[test]
fn test_a_spanning_quoted_compact_key_is_refused() {
    assert_eq!(reads("\"first\n  second\": value\n"), None);
    assert_eq!(reads("- \"first\n  second\": v\n"), None);
}

#[test]
fn test_a_spanning_quoted_flow_key_is_refused() {
    assert_eq!(reads("{\"first\n  second\": 1}\n"), None);
    assert_eq!(reads("{'first\n  second': 1}\n"), None);
}

// PORT7-5, the borders the slice keeps.

#[test]
fn test_an_unterminated_span_is_refused() {
    assert_eq!(reads("key: \"first\n  second\n"), None);
    assert_eq!(reads("key: 'first\n  second\n"), None);
}

#[test]
fn test_a_word_after_the_closing_quote_of_a_span_is_refused() {
    assert_eq!(reads("key: \"first\n  second\" junk\n"), None);
    assert_eq!(
        reads("key: \"first\n  second\" # note\n"),
        want(mapping(vec![(text_key("key"), text("first second"))]))
    );
}

#[test]
fn test_a_marker_line_at_the_head_of_a_crossed_line_is_refused() {
    assert_eq!(reads("a: \"x\n---\ny\"\n"), None);
    assert_eq!(reads("a: \"x\n...\ny\"\n"), None);
    assert_eq!(reads("a: 'x\n---\ny'\n"), None);
    assert_eq!(reads("{\"k\": \"x\n---\ny\"}\n"), None);
    assert_eq!(reads("a: \"x\\\n---\ny\"\n"), None);
}

#[test]
fn test_an_indented_marker_line_inside_a_span_is_content() {
    assert_eq!(
        reads("a: \"x\n  ---\n  y\"\n"),
        want(mapping(vec![(text_key("a"), text("x --- y"))]))
    );
    assert_eq!(
        reads("a: \"x\n---x\ny\"\n"),
        want(mapping(vec![(text_key("a"), text("x ---x y"))]))
    );
    assert_eq!(
        reads("a: \"---\ny\"\n"),
        want(mapping(vec![(text_key("a"), text("--- y"))]))
    );
}

// The stated limit of the slice, the separator characters
// folding as the break they normalize to at the door.

#[test]
fn test_a_line_separator_in_a_span_folds_as_the_break_it_normalizes_to() {
    assert_eq!(
        reads("key: \"a\u{2028}b\"\n"),
        want(mapping(vec![(text_key("key"), text("a b"))]))
    );
    assert_eq!(
        reads("{\"k\": \"a\u{2028}b\"}\n"),
        want(mapping(vec![(text_key("k"), text("a b"))]))
    );
}

#[test]
fn test_a_paragraph_separator_in_a_span_folds_as_the_break_it_normalizes_to() {
    assert_eq!(
        reads("key: \"a\u{2029}b\"\n"),
        want(mapping(vec![(text_key("key"), text("a b"))]))
    );
}
