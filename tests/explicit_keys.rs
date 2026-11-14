//! The pins of Rust port slice 5, explicit keys and the tag
//! repair. Every name PORT5-1 to PORT5-7 carries stands here
//! as a test against the reader, each one able to fail on the
//! reading it pins.

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

fn mapping(pairs: Vec<(&str, YamlValue)>) -> YamlValue {
    YamlValue::Mapping(
        pairs
            .into_iter()
            .map(|(key, value)| (YamlKey::Text(key.to_string()), value))
            .collect(),
    )
}

fn list(items: Vec<YamlValue>) -> YamlValue {
    YamlValue::List(items)
}

#[test]
fn test_an_explicit_key_reads_its_pair() {
    assert_eq!(
        reads("? key\n: value\n"),
        want(mapping(vec![("key", text("value"))]))
    );
}

#[test]
fn test_an_explicit_pair_stands_beside_compact_pairs() {
    assert_eq!(
        reads("a: 1\n? b\n: 2\n"),
        want(mapping(vec![("a", int(1)), ("b", int(2))]))
    );
}

#[test]
fn test_an_explicit_key_with_a_block_sequence_value_reads() {
    assert_eq!(
        reads("? key\n:\n  - 1\n  - 2\n"),
        want(mapping(vec![("key", list(vec![int(1), int(2)]))]))
    );
}

#[test]
fn test_an_explicit_key_spread_over_lines_reads() {
    assert_eq!(
        reads("?\n  key\n: value\n"),
        want(mapping(vec![("key", text("value"))]))
    );
}

#[test]
fn test_a_flow_explicit_key_reads_its_key() {
    assert_eq!(
        reads("{? key: value}\n"),
        want(mapping(vec![("key", text("value"))]))
    );
}

#[test]
fn test_a_flow_explicit_pair_stands_beside_compact_pairs() {
    assert_eq!(
        reads("{a: 1, ? b: 2}\n"),
        want(mapping(vec![("a", int(1)), ("b", int(2))]))
    );
}

#[test]
fn test_an_explicit_null_key_reads() {
    assert_eq!(
        reads("? ~\n: value\n"),
        want(YamlValue::Mapping(vec![(YamlKey::Null, text("value"))]))
    );
}

#[test]
fn test_a_question_mark_against_the_colon_is_scalar_content() {
    assert_eq!(
        reads("?: value\n"),
        want(mapping(vec![("?", text("value"))]))
    );
}

#[test]
fn test_a_block_scalar_explicit_key_reads() {
    assert_eq!(
        reads("? |\n  x\n: 1\n"),
        want(mapping(vec![("x\n", int(1))]))
    );
}

#[test]
fn test_a_str_tag_on_following_lines_reads_an_invalid_date_as_text() {
    assert_eq!(
        reads("key: !!str\n  2026-13-45\n"),
        want(mapping(vec![("key", text("2026-13-45"))]))
    );
}

#[test]
fn test_an_int_tag_on_following_lines_refuses_a_date_spelling() {
    assert_eq!(reads("key: !!int\n  2026-01-01\n"), None);
}

#[test]
fn test_a_timestamp_tag_on_following_lines_refuses_an_invalid_date() {
    assert_eq!(reads("key: !!timestamp\n  2026-13-45\n"), None);
}

#[test]
fn test_a_collection_explicit_key_is_refused() {
    assert_eq!(reads("? [a, b]\n: 1\n"), None);
    assert_eq!(reads("? {a: 1}\n: 1\n"), None);
}

#[test]
fn test_an_explicit_merge_key_merges() {
    assert_eq!(
        reads("base: &b {x: 1}\n? <<\n: *b\n"),
        want(YamlValue::Mapping(vec![
            (YamlKey::Text("x".to_string()), int(1)),
            (
                YamlKey::Text("base".to_string()),
                mapping(vec![("x", int(1))])
            ),
        ]))
    );
}

#[test]
fn test_a_duplicate_explicit_key_stands_at_the_first_position_with_the_last_value() {
    assert_eq!(
        reads("? a\n: 1\nb: 0\n? a\n: 2\n"),
        want(mapping(vec![("a", int(2)), ("b", int(0))]))
    );
}

#[test]
fn test_a_question_line_with_no_colon_line_reads_a_null_value() {
    assert_eq!(
        reads("? foo\n"),
        want(mapping(vec![("foo", YamlValue::Null)]))
    );
    assert_eq!(
        reads("? foo\nbar: 1\n"),
        want(mapping(vec![("foo", YamlValue::Null), ("bar", int(1))]))
    );
}

#[test]
fn test_a_colon_line_with_no_pair_open_is_refused() {
    assert_eq!(reads(": value\n"), None);
}
