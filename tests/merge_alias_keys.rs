//! The pins of Rust port slice 13, the merge spelling
//! through an alias standing as a key. Every name PORT13-1
//! and PORT13-2 carry stands here as a test against the
//! reader, each one able to fail on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT13-1, the reading of the merge spelling through an
// alias standing as a key.

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_at_an_explicit_key() {
    assert_eq!(
        reads("a: &m \"<<\"\n? *m\n: v\n"),
        Some("{'a': '<<', '<<': 'v'}".to_string())
    );
}

#[test]
fn test_a_str_tagged_merge_spelling_reads_through_an_alias_at_an_explicit_key() {
    assert_eq!(
        reads("a: &m !!str <<\n? *m\n: v\n"),
        Some("{'a': '<<', '<<': 'v'}".to_string())
    );
}

#[test]
fn test_a_literal_spelled_merge_spelling_reads_through_an_alias_at_an_explicit_key() {
    assert_eq!(
        reads("a: &m |-\n  <<\n? *m\n: v\n"),
        Some("{'a': '<<', '<<': 'v'}".to_string())
    );
}

#[test]
fn test_a_folded_spelled_merge_spelling_reads_through_an_alias_at_an_explicit_key() {
    assert_eq!(
        reads("a: &m >-\n  <<\n? *m\n: v\n"),
        Some("{'a': '<<', '<<': 'v'}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_at_a_compact_block_key() {
    assert_eq!(
        reads("a: &m \"<<\"\n*m: v\n"),
        Some("{'a': '<<', '<<': 'v'}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_at_a_flow_compact_key() {
    assert_eq!(
        reads("a: &m \"<<\"\nb: {*m: v}\n"),
        Some("{'a': '<<', 'b': {'<<': 'v'}}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_at_a_flow_explicit_key() {
    assert_eq!(
        reads("a: &m \"<<\"\nb: {? *m: v}\n"),
        Some("{'a': '<<', 'b': {'<<': 'v'}}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_nested_inside_flow_collections() {
    assert_eq!(
        reads("a: &m \"<<\"\nb: {x: {*m: v}}\n"),
        Some("{'a': '<<', 'b': {'x': {'<<': 'v'}}}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_key_beside_a_direct_merge() {
    assert_eq!(
        reads("src: &s {p: 1}\na: &m \"<<\"\nouter:\n  <<: *s\n  *m: v\n"),
        Some("{'src': {'p': 1}, 'a': '<<', 'outer': {'p': 1, '<<': 'v'}}".to_string())
    );
}

#[test]
fn test_a_quoted_merge_spelling_reads_through_an_alias_key_duplicated_by_a_direct_string_key() {
    assert_eq!(
        reads("a: &m \"<<\"\n? *m\n: first\n\"<<\": second\n"),
        Some("{'a': '<<', '<<': 'second'}".to_string())
    );
}

// PORT13-2, the borders of the reading.

#[test]
fn test_a_plain_anchored_merge_spelling_still_refuses_at_an_alias_key() {
    assert_eq!(reads("a: &m <<\n? *m\n: v\n"), None);
}

#[test]
fn test_a_merge_tagged_anchored_merge_spelling_still_refuses() {
    assert_eq!(reads("a: &m !!merge <<\n"), None);
}

#[test]
fn test_a_merge_tagged_merge_spelling_in_value_position_still_refuses() {
    assert_eq!(reads("x: !!merge <<\n"), None);
}

#[test]
fn test_an_anchored_set_still_refuses_at_an_alias_key() {
    assert_eq!(reads("a: &s !!set {x, y}\n? *s\n: v\n"), None);
}

#[test]
fn test_an_anchored_list_of_pairs_still_refuses_at_an_alias_key() {
    assert_eq!(reads("a: &p !!omap [x: 1]\n? *p\n: v\n"), None);
}

#[test]
fn test_a_direct_merge_key_with_an_anchor_in_front_of_it_still_merges() {
    assert_eq!(
        reads("a: 1\n&m <<: {b: 2}\n"),
        Some("{'b': 2, 'a': 1}".to_string())
    );
}

#[test]
fn test_a_direct_merge_key_with_the_merge_tag_in_front_of_it_still_merges() {
    assert_eq!(
        reads("a: 1\n!!merge <<: {b: 2}\n"),
        Some("{'b': 2, 'a': 1}".to_string())
    );
}

// The amendment to PORT13-2, the alias key standings at
// an anchored merge key.

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_still_refuses() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: v\n"), None);
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_refuses_where_the_value_can_merge() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: {c: 3}\n"), None);
}
