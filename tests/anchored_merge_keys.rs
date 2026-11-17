//! The pins of Rust port slice 14, the merge standing
//! completed. Every name PORT14-1 and PORT14-2 and
//! PORT14-3 carry stands here as a test against the
//! reader, each one able to fail on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT14-1, the reading of an alias standing as a key to
// an anchored merge key.

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_mapping_value_at_an_explicit_key() {
    // The key on the question mark line, and the key node
    // standing on the lines behind a bare mark.
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n: {c: 3}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
    assert_eq!(
        reads("&m <<: {b: 2}\n?\n  *m\n: {c: 3}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_mapping_value_at_a_compact_key() {
    assert_eq!(
        reads("&m <<: {b: 2}\n*m: {c: 3}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_mapping_value_in_flow() {
    // The compact key inside a flow mapping, and the
    // explicit key inside one.
    assert_eq!(
        reads("{&m <<: {b: 2}, *m: {c: 3}}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
    assert_eq!(
        reads("{&m <<: {b: 2}, ? *m: {c: 3}}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_list_of_mappings() {
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n: [{c: 3}]\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_an_alias_to_a_mapping() {
    assert_eq!(
        reads("src: &s {c: 3}\n&m <<: {b: 2}\n? *m\n: *s\n"),
        Some("{'b': 2, 'c': 3, 'src': {'c': 3}}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_value_carrying_its_own_anchor() {
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n: &v {c: 3}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_block_mapping_behind_the_colon() {
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n:\n  c: 3\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_a_value_carrying_its_own_merge() {
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n: {<<: {e: 5}, c: 3}\n"),
        Some("{'b': 2, 'e': 5, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_twice_in_one_mapping() {
    assert_eq!(
        reads("&m <<: {b: 2}\n? *m\n: {c: 3}\n? *m\n: {d: 4}\n"),
        Some("{'b': 2, 'c': 3, 'd': 4}".to_string())
    );
}

#[test]
fn test_two_alias_keys_to_anchored_merge_keys_merge_in_one_mapping() {
    assert_eq!(
        reads("&m <<: {b: 2}\n&n <<: {e: 5}\n? *m\n: {c: 3}\n? *n\n: {f: 6}\n"),
        Some("{'b': 2, 'e': 5, 'c': 3, 'f': 6}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_a_merge_tagged_key_with_other_text_merges() {
    // A mapping standing as the pair value, and a list of
    // mappings standing as it.
    assert_eq!(
        reads("&m !!merge foo: {b: 2}\n? *m\n: {c: 3}\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
    assert_eq!(
        reads("&m !!merge foo: {b: 2}\n? *m\n: [{c: 3}]\n"),
        Some("{'b': 2, 'c': 3}".to_string())
    );
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_merges_in_a_nested_block_mapping() {
    assert_eq!(
        reads("outer:\n  &m <<: {b: 2}\n  ? *m\n  : {c: 3}\n"),
        Some("{'outer': {'b': 2, 'c': 3}}".to_string())
    );
}

// PORT14-2, the borders of the reading.

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_refuses_a_scalar_value_at_a_compact_key() {
    assert_eq!(reads("&m <<: {b: 2}\n*m: v\n"), None);
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_refuses_an_empty_value() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n:\n"), None);
}

#[test]
fn test_an_alias_key_to_an_anchored_merge_key_refuses_a_list_carrying_a_scalar() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: [{c: 3}, 4]\n"), None);
}

#[test]
fn test_an_alias_key_to_a_merge_tagged_key_with_other_text_refuses_a_scalar_value() {
    assert_eq!(reads("&m !!merge foo: {b: 2}\n? *m\n: v\n"), None);
}

#[test]
fn test_an_alias_key_to_a_quoted_merge_spelling_reads_as_an_ordinary_key_with_a_mapping_value() {
    assert_eq!(
        reads("a: &m \"<<\"\n? *m\n: {c: 3}\n"),
        Some("{'a': '<<', '<<': {'c': 3}}".to_string())
    );
}

#[test]
fn test_an_anchored_merge_key_in_value_or_entry_position_still_refuses() {
    assert_eq!(reads("a: &m <<\n? *m\n: {c: 3}\n"), None);
    assert_eq!(reads("- &m <<\n"), None);
}

// PORT14-3, the anchored merge key standing as a pair
// inside a flow sequence.

#[test]
fn test_an_anchored_merge_key_standing_as_a_flow_sequence_pair_merges() {
    assert_eq!(reads("[&m <<: {b: 2}]\n"), Some("[{'b': 2}]".to_string()));
}

#[test]
fn test_an_anchored_merge_key_standing_as_a_flow_sequence_pair_merges_beside_a_scalar_entry() {
    assert_eq!(
        reads("[&m <<: {b: 2}, z]\n"),
        Some("[{'b': 2}, 'z']".to_string())
    );
}

#[test]
fn test_a_merge_key_anchored_in_a_flow_sequence_merges_through_an_alias_key_in_a_flow_mapping() {
    assert_eq!(
        reads("x: [&m <<: {b: 2}]\ny: {*m: {c: 3}}\n"),
        Some("{'x': [{'b': 2}], 'y': {'c': 3}}".to_string())
    );
}

#[test]
fn test_a_merge_tagged_key_standing_as_a_flow_sequence_pair_merges() {
    // The merge tagged spelling, and the anchored form of
    // it, the anchor binding on the plain token as it
    // binds on the untagged spelling.
    assert_eq!(
        reads("[!!merge <<: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
    assert_eq!(
        reads("[&m !!merge <<: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_an_anchored_quoted_merge_spelling_stands_as_an_ordinary_flow_sequence_pair() {
    // The quoted spelling, and the str tagged spelling.
    assert_eq!(
        reads("[&m \"<<\" : {b: 2}]\n"),
        Some("[{'<<': {'b': 2}}]".to_string())
    );
    assert_eq!(
        reads("[&m !!str <<: {b: 2}]\n"),
        Some("[{'<<': {'b': 2}}]".to_string())
    );
}
