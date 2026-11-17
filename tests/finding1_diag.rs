//! Diagnostic for the Round 14 red team Finding 1, run on a
//! diagnostic branch only and never landed. It asserts the
//! port side of the deciding documents: an alias standing as
//! a key to an anchored merge key refuses in the port where
//! the pair value behind the alias key can merge, the cell
//! the runtime reads by merging.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

#[test]
fn test_diag_alias_key_to_merge_key_with_mapping_value_refuses() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: {c: 3}\n"), None);
}

#[test]
fn test_diag_alias_key_to_merge_key_compact_with_mapping_value_refuses() {
    assert_eq!(reads("&m <<: {b: 2}\n*m: {c: 3}\n"), None);
}

#[test]
fn test_diag_alias_key_to_merge_key_with_list_value_refuses() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: [{c: 3}]\n"), None);
}

#[test]
fn test_diag_alias_key_to_merge_key_with_scalar_value_refuses() {
    assert_eq!(reads("&m <<: {b: 2}\n? *m\n: v\n"), None);
}
