//! The pins of Rust port slice 4, the anchors, aliases, and
//! tags slice. Every name PORT4-1 to PORT4-9 carries stands
//! here as a test against the reader, each one able to fail
//! on the reading it pins.

use deadgate::dates::days_from_civil;
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

// PORT4-1, the standing of an anchor.

#[test]
fn test_an_anchor_on_a_mapping_value_binds_the_scalar_standing_after_it() {
    assert_eq!(
        reads("a: &x 5\nb: *x\n"),
        want(mapping(vec![("a", int(5)), ("b", int(5))]))
    );
}

#[test]
fn test_an_anchor_on_an_empty_mapping_value_binds_the_nested_block_collection() {
    let inner = mapping(vec![("k", int(1))]);
    assert_eq!(
        reads("a: &x\n  k: 1\nb: *x\n"),
        want(mapping(vec![("a", inner.clone()), ("b", inner)]))
    );
}

#[test]
fn test_an_anchor_on_a_sequence_entry_binds_its_value() {
    assert_eq!(
        reads("- &x 5\n- *x\n"),
        want(list(vec![int(5), int(5)]))
    );
}

#[test]
fn test_an_anchor_at_the_root_binds_the_document_value() {
    assert_eq!(reads("&x 5\n"), want(int(5)));
}

#[test]
fn test_an_anchor_on_a_flow_collection_binds_the_whole_collection() {
    let whole = list(vec![int(1), int(2)]);
    assert_eq!(
        reads("a: &x [1, 2]\nb: *x\n"),
        want(mapping(vec![("a", whole.clone()), ("b", whole)]))
    );
}

#[test]
fn test_an_anchor_behind_a_document_opener_binds_the_root_value() {
    assert_eq!(reads("--- &x 5\n"), want(int(5)));
}

// PORT4-2, the refusals standing on anchors.

#[test]
fn test_a_duplicate_anchor_is_refused() {
    assert_eq!(reads("a: &x 1\nb: &x 2\n"), None);
}

#[test]
fn test_an_anchor_with_no_name_is_refused() {
    assert_eq!(reads("a: & 5\n"), None);
}

#[test]
fn test_an_anchor_name_carrying_a_character_outside_the_grammar_is_refused() {
    assert_eq!(reads("a: &x.y 5\n"), None);
}

#[test]
fn test_two_anchors_standing_on_one_node_are_refused() {
    assert_eq!(reads("a: &x &y 5\n"), None);
}

#[test]
fn test_two_tags_standing_on_one_node_are_refused() {
    assert_eq!(reads("a: !!str !!int 5\n"), None);
}

// PORT4-3, aliases.

#[test]
fn test_an_alias_reads_as_a_copy_of_the_anchored_value() {
    let whole = list(vec![int(1), int(2)]);
    assert_eq!(
        reads("a: &x [1, 2]\nb: *x\n"),
        want(mapping(vec![("a", whole.clone()), ("b", whole)]))
    );
}

#[test]
fn test_an_undefined_alias_is_refused() {
    assert_eq!(reads("a: *x\n"), None);
}

#[test]
fn test_an_alias_standing_before_its_anchor_is_refused() {
    assert_eq!(reads("a: *x\nb: &x 1\n"), None);
}

// PORT4-4, merge keys standing.

#[test]
fn test_a_merge_key_standing_first_expands_ahead_of_the_explicit_pairs() {
    assert_eq!(
        reads("s: &s {x: 1, y: 1}\nb:\n  <<: *s\n  y: 2\n"),
        want(mapping(vec![
            ("s", mapping(vec![("x", int(1)), ("y", int(1))])),
            ("b", mapping(vec![("x", int(1)), ("y", int(2))])),
        ]))
    );
}

#[test]
fn test_a_merge_key_standing_last_still_expands_ahead_of_the_explicit_pairs() {
    assert_eq!(
        reads("s: &s {x: 1, y: 1}\nb:\n  y: 2\n  <<: *s\n"),
        want(mapping(vec![
            ("s", mapping(vec![("x", int(1)), ("y", int(1))])),
            ("b", mapping(vec![("x", int(1)), ("y", int(2))])),
        ]))
    );
}

#[test]
fn test_a_merge_list_lets_the_earlier_listed_source_win_its_repeated_keys() {
    assert_eq!(
        reads("a: &a {x: 1, y: 1}\nb: &b {y: 2, z: 2}\nc:\n  <<: [*a, *b]\n  y: 3\n"),
        want(mapping(vec![
            ("a", mapping(vec![("x", int(1)), ("y", int(1))])),
            ("b", mapping(vec![("y", int(2)), ("z", int(2))])),
            (
                "c",
                mapping(vec![("y", int(3)), ("z", int(2)), ("x", int(1))])
            ),
        ]))
    );
}

#[test]
fn test_an_explicit_pair_repeated_around_a_merge_takes_the_last_value_at_the_flattened_position()
{
    assert_eq!(
        reads("s: &s {x: 1, y: 1}\nb:\n  x: 0\n  <<: *s\n  x: 9\n"),
        want(mapping(vec![
            ("s", mapping(vec![("x", int(1)), ("y", int(1))])),
            ("b", mapping(vec![("x", int(9)), ("y", int(1))])),
        ]))
    );
}

#[test]
fn test_a_merge_expansion_leaves_the_anchored_source_mapping_unchanged() {
    assert_eq!(
        reads("s: &s {x: 1}\nb:\n  <<: *s\n  y: 2\nc: *s\n"),
        want(mapping(vec![
            ("s", mapping(vec![("x", int(1))])),
            ("b", mapping(vec![("x", int(1)), ("y", int(2))])),
            ("c", mapping(vec![("x", int(1))])),
        ]))
    );
}

// PORT4-5, merge refusals and non-merges.

#[test]
fn test_a_quoted_merge_key_reads_as_an_ordinary_key() {
    assert_eq!(
        reads("s: &s {x: 1}\nb:\n  '<<': *s\n"),
        want(mapping(vec![
            ("s", mapping(vec![("x", int(1))])),
            ("b", mapping(vec![("<<", mapping(vec![("x", int(1))]))])),
        ]))
    );
}

#[test]
fn test_a_scalar_merge_source_is_refused() {
    assert_eq!(reads("b:\n  <<: 5\n"), None);
}

#[test]
fn test_a_null_merge_source_is_refused() {
    assert_eq!(reads("b:\n  <<: ~\n"), None);
}

#[test]
fn test_a_merge_list_holding_a_non_mapping_is_refused() {
    assert_eq!(reads("s: &s {x: 1}\nb:\n  <<: [*s, 5]\n"), None);
}

#[test]
fn test_an_empty_merge_value_is_refused() {
    assert_eq!(reads("b:\n  <<:\n"), None);
}

#[test]
fn test_the_plain_merge_spelling_standing_in_value_position_is_refused() {
    assert_eq!(reads("a: <<\n"), None);
}

// PORT4-6, scalar tags.

#[test]
fn test_the_str_tag_reads_a_number_spelling_and_a_date_spelling_as_text() {
    assert_eq!(
        reads("a: !!str 5\n"),
        want(mapping(vec![("a", text("5"))]))
    );
    assert_eq!(
        reads("a: !!str 2026-10-14\n"),
        want(mapping(vec![("a", text("2026-10-14"))]))
    );
}

#[test]
fn test_the_int_tag_reads_the_hex_and_sexagesimal_and_octal_spellings() {
    assert_eq!(
        reads("a: !!int 0x1F\n"),
        want(mapping(vec![("a", int(31))]))
    );
    assert_eq!(
        reads("a: !!int 1:99\n"),
        want(mapping(vec![("a", int(159))]))
    );
    assert_eq!(
        reads("a: !!int 017\n"),
        want(mapping(vec![("a", int(15))]))
    );
}

#[test]
fn test_the_float_tag_reads_a_sexagesimal_spelling_and_infinity() {
    assert_eq!(
        reads("a: !!float 1:30.5\n"),
        want(mapping(vec![("a", YamlValue::Float(90.5))]))
    );
    assert_eq!(
        reads("a: !!float .inf\n"),
        want(mapping(vec![("a", YamlValue::Float(f64::INFINITY))]))
    );
}

#[test]
fn test_the_timestamp_tag_reads_a_date_and_a_datetime() {
    assert_eq!(
        reads("a: !!timestamp 2001-2-3\n"),
        want(mapping(vec![(
            "a",
            YamlValue::Date(days_from_civil(2001, 2, 3))
        )]))
    );
    let micros = days_from_civil(2001, 12, 15) as i128 * 86_400_000_000
        + (2 * 3600 + 59 * 60 + 43) as i128 * 1_000_000;
    assert_eq!(
        reads("a: !!timestamp 2001-12-15T02:59:43Z\n"),
        want(mapping(vec![("a", YamlValue::AwareDateTime(micros, 0))]))
    );
}

#[test]
fn test_a_tagged_empty_scalar_under_str_reads_as_the_empty_string() {
    assert_eq!(
        reads("a: !!str\n"),
        want(mapping(vec![("a", text(""))]))
    );
}

// PORT4-7, collection tags and unknown tags.

#[test]
fn test_the_seq_tag_on_a_scalar_node_is_refused() {
    assert_eq!(reads("a: !!seq hello\n"), None);
}

#[test]
fn test_the_str_tag_on_a_collection_node_is_refused() {
    assert_eq!(reads("a: !!str [1]\n"), None);
}

#[test]
fn test_an_unknown_tag_is_refused() {
    assert_eq!(reads("a: !foo 5\n"), None);
}

#[test]
fn test_a_tag_directive_prefix_expands_a_standard_tag() {
    assert_eq!(
        reads("%TAG !e! tag:yaml.org,2002:\n---\na: !e!str 5\n"),
        want(mapping(vec![("a", text("5"))]))
    );
}

#[test]
fn test_a_tag_carrying_no_name_is_refused() {
    assert_eq!(reads("a: !<> 5\n"), None);
}

// PORT4-8, properties together and the block borders.

#[test]
fn test_a_block_head_behind_an_anchor_reads() {
    assert_eq!(
        reads("a: &x |\n  y\n"),
        want(mapping(vec![("a", text("y\n"))]))
    );
}

#[test]
fn test_a_block_head_behind_a_tag_reads() {
    assert_eq!(
        reads("a: !!str |\n  y\n"),
        want(mapping(vec![("a", text("y\n"))]))
    );
}

#[test]
fn test_a_tab_inside_an_anchored_block_stands_in_the_value() {
    assert_eq!(
        reads("a: &x |\n  x\ty\n"),
        want(mapping(vec![("a", text("x\ty\n"))]))
    );
}

#[test]
fn test_the_int_tag_on_a_block_scalar_converts_the_assembled_text() {
    assert_eq!(
        reads("a: !!int |\n  42\n"),
        want(mapping(vec![("a", int(42))]))
    );
}

// PORT4-9, the borders kept.

#[test]
fn test_an_anchor_in_key_position_is_refused() {
    assert_eq!(reads("a: 1\n&x b: 2\n"), None);
}

#[test]
fn test_a_tag_in_key_position_is_refused() {
    assert_eq!(reads("{!!str k: v}\n"), None);
}

#[test]
fn test_a_recursive_alias_is_refused() {
    assert_eq!(reads("a: &x [*x]\n"), None);
}

#[test]
fn test_an_alias_in_key_position_is_refused() {
    assert_eq!(reads("a: &x 1\n*x: 2\n"), None);
}

#[test]
fn test_a_tag_and_both_properties_and_a_block_head_behind_a_document_opener_reads() {
    assert_eq!(reads("--- !!str &x |\n  hi\n"), want(text("hi\n")));
}

#[test]
fn test_a_document_past_the_expansion_bound_is_refused() {
    let mut doc = String::from("a: &x 1\nb: [");
    for index in 0..4001 {
        if index > 0 {
            doc.push_str(", ");
        }
        doc.push_str("*x");
    }
    doc.push_str("]\n");
    assert_eq!(reads(&doc), None);
}
