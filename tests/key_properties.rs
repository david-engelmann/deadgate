//! The pins of Rust port slice 6, the key properties slice.
//! Every name PORT6-1 to PORT6-5 carries stands here as a
//! test against the reader, each one able to fail on the
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

fn mapping(pairs: Vec<(YamlKey, YamlValue)>) -> YamlValue {
    YamlValue::Mapping(pairs)
}

fn text_key(key: &str) -> YamlKey {
    YamlKey::Text(key.to_string())
}

fn list(items: Vec<YamlValue>) -> YamlValue {
    YamlValue::List(items)
}

// PORT6-1, properties in front of a compact block key.

#[test]
fn test_an_anchor_in_front_of_a_block_key_reads() {
    assert_eq!(
        reads("a: 1\n&x b: 2\n"),
        want(mapping(vec![
            (text_key("a"), int(1)),
            (text_key("b"), int(2)),
        ]))
    );
}

#[test]
fn test_a_tag_in_front_of_a_block_key_converts_the_key() {
    assert_eq!(
        reads("!!str 5: v\n"),
        want(mapping(vec![(text_key("5"), text("v"))]))
    );
    assert_eq!(
        reads("5: v\n"),
        want(mapping(vec![(YamlKey::Int(5), text("v"))]))
    );
}

#[test]
fn test_both_properties_in_front_of_a_block_key_read_in_either_order() {
    let wanted = want(mapping(vec![
        (text_key("5"), text("v")),
        (text_key("other"), text("5")),
    ]));
    assert_eq!(reads("!!str &a 5: v\nother: *a\n"), wanted);
    assert_eq!(reads("&a !!str 5: v\nother: *a\n"), wanted);
}

#[test]
fn test_an_anchor_in_front_of_a_quoted_key_reads() {
    assert_eq!(
        reads("&a 'key': v\nother: *a\n"),
        want(mapping(vec![
            (text_key("key"), text("v")),
            (text_key("other"), text("key")),
        ]))
    );
}

// PORT6-2, properties in the remaining key positions.

#[test]
fn test_an_anchor_in_front_of_a_flow_key_reads() {
    assert_eq!(
        reads("{&a k: v, x: *a}\n"),
        want(mapping(vec![
            (text_key("k"), text("v")),
            (text_key("x"), text("k")),
        ]))
    );
}

#[test]
fn test_a_tag_in_front_of_a_flow_key_converts_the_key() {
    assert_eq!(
        reads("{!!str 5: v}\n"),
        want(mapping(vec![(text_key("5"), text("v"))]))
    );
}

#[test]
fn test_a_property_in_front_of_an_explicit_key_reads() {
    assert_eq!(
        reads("? !!str 5\n: v\n"),
        want(mapping(vec![(text_key("5"), text("v"))]))
    );
    assert_eq!(
        reads("? &a k\n: v\nother: *a\n"),
        want(mapping(vec![
            (text_key("k"), text("v")),
            (text_key("other"), text("k")),
        ]))
    );
}

#[test]
fn test_a_property_in_front_of_a_block_scalar_explicit_key_reads() {
    assert_eq!(
        reads("? !!str |\n  k\n: v\n"),
        want(mapping(vec![(text_key("k\n"), text("v"))]))
    );
}

// PORT6-3, the reading of a key under a tag.

#[test]
fn test_a_str_tag_on_a_key_keeps_the_text_as_the_key() {
    assert_eq!(
        reads("!!str 2026-01-01: v\n"),
        want(mapping(vec![(text_key("2026-01-01"), text("v"))]))
    );
}

#[test]
fn test_an_int_tag_on_a_quoted_key_converts() {
    assert_eq!(
        reads("!!int \"5\": v\n"),
        want(mapping(vec![(YamlKey::Int(5), text("v"))]))
    );
}

#[test]
fn test_a_null_tag_on_a_key_reads_the_null_key() {
    assert_eq!(
        reads("!!null 'x': v\n"),
        want(mapping(vec![(YamlKey::Null, text("v"))]))
    );
}

#[test]
fn test_an_anchor_on_the_merge_spelling_is_refused() {
    assert_eq!(reads("&a <<: v\n"), None);
}

#[test]
fn test_a_key_under_an_unknown_tag_is_refused() {
    assert_eq!(reads("!<tag:x> k: v\n"), None);
}

// PORT6-4, anchors bound at keys and aliases standing as keys.

#[test]
fn test_an_anchor_on_a_key_binds_the_key_node() {
    assert_eq!(
        reads("&a k: v\nother: *a\n"),
        want(mapping(vec![
            (text_key("k"), text("v")),
            (text_key("other"), text("k")),
        ]))
    );
}

#[test]
fn test_an_alias_standing_as_a_block_key_reads() {
    assert_eq!(
        reads("base: &a name\nother:\n  *a : v\n"),
        want(mapping(vec![
            (text_key("base"), text("name")),
            (
                text_key("other"),
                mapping(vec![(text_key("name"), text("v"))]),
            ),
        ]))
    );
}

#[test]
fn test_an_alias_standing_as_a_flow_key_reads() {
    assert_eq!(
        reads("defs: &a name\nm: {*a : 1}\n"),
        want(mapping(vec![
            (text_key("defs"), text("name")),
            (text_key("m"), mapping(vec![(text_key("name"), int(1))]),),
        ]))
    );
}

#[test]
fn test_an_alias_to_a_collection_standing_as_a_key_is_refused() {
    assert_eq!(reads("base: &a [1, 2]\nother:\n  *a : v\n"), None);
}

#[test]
fn test_an_undefined_alias_standing_as_a_key_is_refused() {
    assert_eq!(reads("m:\n  *nope : 1\n"), None);
}

// PORT6-5, the borders this slice keeps.

#[test]
fn test_a_duplicate_formed_by_conversion_stands_at_the_first_position() {
    assert_eq!(
        reads("5: a\n!!int \"5\": b\n"),
        want(mapping(vec![(YamlKey::Int(5), text("b"))]))
    );
    assert_eq!(
        reads("5: a\n!!str 5: b\n"),
        want(mapping(vec![
            (YamlKey::Int(5), text("a")),
            (text_key("5"), text("b")),
        ]))
    );
}

#[test]
fn test_the_binary_tag_in_key_position_is_refused() {
    assert_eq!(reads("!!binary k: v\n"), None);
    assert_eq!(reads("!!seq k: v\n"), None);
}

#[test]
fn test_an_alias_key_to_the_merge_spelling_is_refused() {
    // The stated narrow limit of PORT6-4's reading path, the
    // plain anchored merge spelling dying in the runtime's
    // constructor.
    assert_eq!(reads("a: &x <<\n? *x\n: v\n"), None);
}

#[test]
fn test_a_flow_sequence_pair_key_takes_properties() {
    assert_eq!(
        reads("[{&a k: v}, *a]\n"),
        want(list(vec![
            mapping(vec![(text_key("k"), text("v"))]),
            text("k"),
        ]))
    );
    assert_eq!(
        reads("[{!!str 5: v}]\n"),
        want(list(vec![mapping(vec![(text_key("5"), text("v"))])]))
    );
}
