//! The pins of Rust port slice 9, the collection and binary
//! tags. Every name PORT9-1 to PORT9-6 carries stands here as
//! a test against the reader, each one able to fail on the
//! reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT9-1, the binary tag.

#[test]
fn test_a_binary_scalar_reads_as_bytes() {
    assert_eq!(
        reads("a: !!binary aGk=\n"),
        Some("{'a': b'hi'}".to_string())
    );
}

#[test]
fn test_a_binary_scalar_with_spaces_reads() {
    assert_eq!(
        reads("a: !!binary aG k=\n"),
        Some("{'a': b'hi'}".to_string())
    );
}

#[test]
fn test_a_binary_block_scalar_reads() {
    assert_eq!(
        reads("a: !!binary |\n  aGk=\n"),
        Some("{'a': b'hi'}".to_string())
    );
}

#[test]
fn test_a_binary_scalar_in_flow_reads() {
    assert_eq!(
        reads("{k: !!binary aGk=}\n"),
        Some("{'k': b'hi'}".to_string())
    );
}

#[test]
fn test_a_binary_key_reads() {
    assert_eq!(reads("!!binary aGk=: 1\n"), Some("{b'hi': 1}".to_string()));
}

#[test]
fn test_a_binary_with_bad_padding_is_refused() {
    assert_eq!(reads("a: !!binary aGk\n"), None);
}

#[test]
fn test_an_empty_binary_reads_as_empty_bytes() {
    assert_eq!(reads("a: !!binary ''\n"), Some("{'a': b''}".to_string()));
}

#[test]
fn test_an_anchored_binary_reads_through_its_alias() {
    assert_eq!(
        reads("a: &x !!binary aGk=\nb: *x\n"),
        Some("{'a': b'hi', 'b': b'hi'}".to_string())
    );
}

// PORT9-2, the spelling of the bytes.

#[test]
fn test_bytes_repr_escapes_a_nul_byte() {
    assert_eq!(
        reads("a: !!binary AA==\n"),
        Some("{'a': b'\\x00'}".to_string())
    );
}

#[test]
fn test_bytes_repr_prefers_the_double_quote_when_the_bytes_carry_a_single_quote() {
    assert_eq!(
        reads("a: !!binary Jw==\n"),
        Some("{'a': b\"'\"}".to_string())
    );
}

#[test]
fn test_bytes_repr_escapes_the_backslash() {
    assert_eq!(
        reads("a: !!binary XA==\n"),
        Some("{'a': b'\\\\'}".to_string())
    );
}

// PORT9-3, the set tag.

#[test]
fn test_a_set_reads_its_keys_as_members() {
    assert_eq!(
        reads("a: !!set\n  x: ~\n  y: ~\n"),
        Some("{'a': {'x', 'y'}}".to_string())
    );
}

#[test]
fn test_a_set_discards_the_values_beside_its_keys() {
    assert_eq!(
        reads("a: !!set\n  x: 1\n"),
        Some("{'a': {'x'}}".to_string())
    );
}

#[test]
fn test_a_set_collapses_a_duplicate_member() {
    assert_eq!(
        reads("a: !!set\n  x: ~\n  x: ~\n"),
        Some("{'a': {'x'}}".to_string())
    );
}

#[test]
fn test_a_set_counts_a_number_and_its_boolean_as_one_member() {
    assert_eq!(
        reads("a: !!set {1, true}\n"),
        Some("{'a': {1}}".to_string())
    );
}

#[test]
fn test_a_set_in_flow_reads() {
    assert_eq!(
        reads("a: !!set {x, y}\n"),
        Some("{'a': {'x', 'y'}}".to_string())
    );
}

#[test]
fn test_an_empty_set_reads() {
    assert_eq!(reads("a: !!set {}\n"), Some("{'a': set()}".to_string()));
}

#[test]
fn test_a_set_with_an_unhashable_member_is_refused() {
    assert_eq!(reads("a: !!set\n  ? [1]\n"), None);
}

#[test]
fn test_a_set_over_a_scalar_is_refused() {
    assert_eq!(reads("a: !!set word\n"), None);
}

#[test]
fn test_a_set_standing_as_a_key_is_refused() {
    assert_eq!(reads("? !!set {a}\n: 1\n"), None);
}

// PORT9-4, the ordered map and pairs tags.

#[test]
fn test_an_ordered_map_reads_as_pairs() {
    assert_eq!(
        reads("a: !!omap\n  - x: 1\n  - y: 2\n"),
        Some("{'a': [('x', 1), ('y', 2)]}".to_string())
    );
}

#[test]
fn test_pairs_read_as_pairs() {
    assert_eq!(
        reads("a: !!pairs\n  - x: 1\n  - y: 2\n"),
        Some("{'a': [('x', 1), ('y', 2)]}".to_string())
    );
}

#[test]
fn test_pairs_keep_duplicate_keys() {
    assert_eq!(
        reads("a: !!pairs\n  - x: 1\n  - x: 2\n"),
        Some("{'a': [('x', 1), ('x', 2)]}".to_string())
    );
}

#[test]
fn test_an_ordered_map_entry_with_two_pairs_is_refused() {
    assert_eq!(reads("a: !!omap\n  - {x: 1, y: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_that_is_not_a_mapping_is_refused() {
    assert_eq!(reads("a: !!omap\n  - 5\n"), None);
}

#[test]
fn test_pairs_over_a_mapping_are_refused() {
    assert_eq!(reads("a: !!pairs {x: 1}\n"), None);
}

#[test]
fn test_a_list_of_pairs_standing_as_a_key_is_refused() {
    assert_eq!(reads("? !!omap [{x: 1}]\n: 2\n"), None);
}

#[test]
fn test_a_pairs_entry_whose_key_is_a_container_is_refused() {
    assert_eq!(reads("a: !!pairs\n  - !!omap [{x: 1}]: 2\n"), None);
}

// PORT9-5, the recorded spelling of a set.

#[test]
fn test_a_set_spells_its_members_sorted() {
    assert_eq!(
        reads("a: !!set {y, x}\n"),
        Some("{'a': {'x', 'y'}}".to_string())
    );
}

#[test]
fn test_an_empty_set_spells_as_the_runtime_spells_it() {
    assert_eq!(reads("!!set {}\n"), Some("set()".to_string()));
}

// PORT9-6, the borders.

#[test]
fn test_an_unknown_tag_still_refuses() {
    assert_eq!(reads("a: !mytag 12\n"), None);
}

#[test]
fn test_a_sequence_tag_over_a_mapping_still_refuses() {
    assert_eq!(reads("a: !!seq {x: 1}\n"), None);
}
