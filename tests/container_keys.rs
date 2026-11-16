//! The pins of Rust port slice 10, pairs keys of every
//! value. Every name PORT10-1 to PORT10-4 carries stands
//! here as a test against the reader, each one able to fail
//! on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT10-1, the pairs key reading.

#[test]
fn test_a_pairs_entry_whose_key_is_a_sequence_reads() {
    assert_eq!(
        reads("a: !!pairs\n  - [1, 2]: v\n"),
        Some("{'a': [([1, 2], 'v')]}".to_string())
    );
}

#[test]
fn test_a_pairs_entry_whose_key_is_a_mapping_reads() {
    assert_eq!(
        reads("a: !!pairs\n  - {x: 1}: v\n"),
        Some("{'a': [({'x': 1}, 'v')]}".to_string())
    );
}

#[test]
fn test_a_pairs_entry_whose_key_is_a_set_reads() {
    assert_eq!(
        reads("a: !!pairs\n  - !!set {p, q}: v\n"),
        Some("{'a': [({'p', 'q'}, 'v')]}".to_string())
    );
}

#[test]
fn test_a_pairs_entry_whose_key_is_a_list_of_pairs_reads() {
    assert_eq!(
        reads("a: !!pairs\n  - !!omap [x: 1]: v\n"),
        Some("{'a': [([('x', 1)], 'v')]}".to_string())
    );
}

#[test]
fn test_an_ordered_map_entry_whose_key_is_a_sequence_reads() {
    assert_eq!(
        reads("a: !!omap\n  - [1, 2]: v\n"),
        Some("{'a': [([1, 2], 'v')]}".to_string())
    );
}

#[test]
fn test_a_pairs_key_holding_a_list_of_pairs_with_a_container_key_reads() {
    assert_eq!(
        reads("a: !!pairs\n  - !!omap [[9]: deep]: top\n"),
        Some("{'a': [([([9], 'deep')], 'top')]}".to_string())
    );
}

#[test]
fn test_duplicate_container_keys_stand_in_a_pairs_list() {
    assert_eq!(
        reads("a: !!pairs\n  - [1, 2]: x\n  - [1, 2]: y\n"),
        Some("{'a': [([1, 2], 'x'), ([1, 2], 'y')]}".to_string())
    );
}

#[test]
fn test_an_empty_sequence_and_an_empty_mapping_read_as_pairs_keys() {
    assert_eq!(
        reads("a: !!pairs\n  - []: v\n"),
        Some("{'a': [([], 'v')]}".to_string())
    );
    assert_eq!(
        reads("a: !!pairs\n  - {}: v\n"),
        Some("{'a': [({}, 'v')]}".to_string())
    );
}

// PORT10-2, the borders this slice keeps.

#[test]
fn test_a_container_key_in_a_plain_mapping_still_refuses() {
    assert_eq!(reads("? [1, 2]\n: v\n"), None);
}

#[test]
fn test_a_container_key_in_a_pairs_value_still_refuses() {
    assert_eq!(reads("a: !!pairs\n  - k: {[1, 2]: v}\n"), None);
}

#[test]
fn test_a_container_member_of_a_set_still_refuses() {
    assert_eq!(reads("a: !!set\n  ? [1]\n"), None);
}

#[test]
fn test_a_list_of_pairs_standing_as_a_key_still_refuses() {
    assert_eq!(reads("? !!pairs [x: 1]\n: 2\n"), None);
}

#[test]
fn test_a_two_pair_entry_with_a_container_key_is_refused() {
    assert_eq!(reads("a: !!omap\n  - {[1]: x, y: 2}\n"), None);
}

// PORT10-3, the spelling of a container key in a pair.

#[test]
fn test_a_sequence_key_in_a_pair_spells_as_a_list() {
    assert_eq!(
        reads("!!pairs\n- [1, 'two', true]: v\n"),
        Some("[([1, 'two', True], 'v')]".to_string())
    );
}

#[test]
fn test_a_mapping_key_in_a_pair_spells_as_a_mapping() {
    assert_eq!(
        reads("!!pairs\n- {x: [1, 2], y: {z: 3}}: v\n"),
        Some("[({'x': [1, 2], 'y': {'z': 3}}, 'v')]".to_string())
    );
}

#[test]
fn test_a_set_key_in_a_pair_spells_with_sorted_members() {
    assert_eq!(
        reads("!!pairs\n- !!set {q, p, a}: v\n"),
        Some("[({'a', 'p', 'q'}, 'v')]".to_string())
    );
}

// PORT10-4, anchors and aliases and merge keys over the
// widened keys.

#[test]
fn test_an_alias_of_a_sequence_stands_as_a_pairs_key() {
    assert_eq!(
        reads("defs: &k [7, 8]\nuse: !!pairs\n  - *k: v\n"),
        Some("{'defs': [7, 8], 'use': [([7, 8], 'v')]}".to_string())
    );
}

#[test]
fn test_a_merged_mapping_stands_as_a_pairs_key() {
    assert_eq!(
        reads("src: &m {a: 1}\nuse: !!pairs [{? {<<: *m, b: 2}: v}]\n"),
        Some("{'src': {'a': 1}, 'use': [({'a': 1, 'b': 2}, 'v')]}".to_string())
    );
}

#[test]
fn test_a_set_key_with_an_unhashable_member_is_refused() {
    assert_eq!(reads("a: !!pairs\n  - !!set {? !!pairs [x: y]}: v\n"), None);
}
