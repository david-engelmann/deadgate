//! The pins of Rust port slice 12, hashes inside flow plain
//! scalars. Every name PORT12-1 and PORT12-2 carry stands
//! here as a test against the reader, each one able to fail
//! on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT12-1, the reading of hashes inside flow plain scalars.

#[test]
fn test_a_flow_plain_scalar_carrying_a_hash_reads() {
    assert_eq!(reads("[a#b, x]"), Some("['a#b', 'x']".to_string()));
}

#[test]
fn test_a_hash_carrying_scalar_reads_as_a_flow_mapping_value() {
    assert_eq!(
        reads("a: [a#b, x]"),
        Some("{'a': ['a#b', 'x']}".to_string())
    );
}

#[test]
fn test_a_hash_carrying_scalar_reads_as_a_flow_key() {
    assert_eq!(reads("{a#b: 1}"), Some("{'a#b': 1}".to_string()));
}

#[test]
fn test_a_hash_at_a_plain_scalars_end_reads() {
    assert_eq!(reads("[a#, x]"), Some("['a#', 'x']".to_string()));
}

#[test]
fn test_a_hash_inside_a_plain_scalar_spanning_words_reads() {
    assert_eq!(reads("[a# b, x]"), Some("['a# b', 'x']".to_string()));
}

#[test]
fn test_hashes_inside_flow_plain_scalars_read_across_a_mapping() {
    assert_eq!(reads("{a#b: c#d}"), Some("{'a#b': 'c#d'}".to_string()));
}

#[test]
fn test_a_nested_flow_collection_carrying_a_hash_reads() {
    assert_eq!(reads("[[a#b], x]"), Some("[['a#b'], 'x']".to_string()));
}

#[test]
fn test_a_flow_collection_spanning_lines_carries_a_hash() {
    assert_eq!(
        reads("a: [[x, a#b],\n  y]"),
        Some("{'a': [['x', 'a#b'], 'y']}".to_string())
    );
}

#[test]
fn test_a_hash_carrying_scalar_reads_beside_a_quoted_scalar() {
    assert_eq!(
        reads("['quoted', a#b]"),
        Some("['quoted', 'a#b']".to_string())
    );
}

#[test]
fn test_a_comment_after_a_hash_carrying_flow_collection_reads() {
    assert_eq!(
        reads("[a#b, x] # comment"),
        Some("['a#b', 'x']".to_string())
    );
}

#[test]
fn test_a_hash_behind_a_colon_inside_a_flow_plain_scalar_reads() {
    assert_eq!(reads("[a:#b]"), Some("['a:#b']".to_string()));
}

#[test]
fn test_a_hash_carrying_scalar_behind_an_anchor_reads() {
    assert_eq!(reads("[&a a#b, x]"), Some("['a#b', 'x']".to_string()));
}

// PORT12-2, the borders this slice keeps.

#[test]
fn test_a_comment_swallowing_a_closing_bracket_still_refuses() {
    assert_eq!(reads("[a #b, x]"), None);
}

#[test]
fn test_an_unterminated_flow_collection_carrying_a_hash_still_refuses() {
    assert_eq!(reads("[a#b, x"), None);
}

#[test]
fn test_a_hash_at_an_entry_start_still_opens_a_comment() {
    assert_eq!(reads("[a, #b\n x]"), Some("['a', 'x']".to_string()));
}

#[test]
fn test_a_comment_behind_a_blank_inside_a_flow_entry_still_reads() {
    assert_eq!(reads("[a #b\n, x]"), Some("['a', 'x']".to_string()));
}

#[test]
fn test_a_bracket_standing_inside_a_comment_opens_nothing() {
    assert_eq!(reads("[a # [\n, x]"), Some("['a', 'x']".to_string()));
}
