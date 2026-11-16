//! The pins of Rust port slice 11, quotes inside flow plain
//! scalars. Every name PORT11-1 and PORT11-2 carry stands
//! here as a test against the reader, each one able to fail
//! on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT11-1, the reading of quotes inside flow plain scalars.

#[test]
fn test_a_flow_plain_scalar_carrying_an_apostrophe_reads() {
    assert_eq!(reads("[don't, x]"), Some("[\"don't\", 'x']".to_string()));
}

#[test]
fn test_an_apostrophe_carrying_scalar_reads_as_a_flow_mapping_value() {
    assert_eq!(
        reads("a: [don't, x]"),
        Some("{'a': [\"don't\", 'x']}".to_string())
    );
}

#[test]
fn test_an_apostrophe_carrying_scalar_reads_as_a_flow_key() {
    assert_eq!(reads("{don't: 1}"), Some("{\"don't\": 1}".to_string()));
}

#[test]
fn test_a_flow_plain_scalar_carrying_a_double_quote_reads() {
    assert_eq!(reads("[a\"b, x]"), Some("['a\"b', 'x']".to_string()));
}

#[test]
fn test_an_apostrophe_at_a_plain_scalars_end_reads() {
    assert_eq!(reads("[don', x]"), Some("[\"don'\", 'x']".to_string()));
}

#[test]
fn test_doubled_apostrophes_inside_a_flow_plain_scalar_read() {
    assert_eq!(reads("[it''s, x]"), Some("[\"it''s\", 'x']".to_string()));
}

#[test]
fn test_quotes_inside_a_plain_scalar_spanning_words_read() {
    assert_eq!(
        reads("[say 'hi' there, x]"),
        Some("[\"say 'hi' there\", 'x']".to_string())
    );
}

#[test]
fn test_a_flow_collection_spanning_lines_carries_an_apostrophe() {
    assert_eq!(
        reads("a: [don't,\n  x]"),
        Some("{'a': [\"don't\", 'x']}".to_string())
    );
}

#[test]
fn test_a_nested_flow_collection_carrying_an_apostrophe_reads() {
    assert_eq!(
        reads("[[don't], x]"),
        Some("[[\"don't\"], 'x']".to_string())
    );
}

#[test]
fn test_an_apostrophe_carrying_scalar_reads_beside_a_quoted_scalar() {
    assert_eq!(
        reads("['quoted', don't]"),
        Some("['quoted', \"don't\"]".to_string())
    );
}

#[test]
fn test_a_comment_after_an_apostrophe_carrying_flow_collection_reads() {
    assert_eq!(
        reads("[don't, x] # comment"),
        Some("[\"don't\", 'x']".to_string())
    );
}

// PORT11-2, the borders this slice keeps.

#[test]
fn test_a_quoted_scalar_at_an_entry_start_still_reads_as_quoted() {
    assert_eq!(reads("['quoted', x]"), Some("['quoted', 'x']".to_string()));
}

#[test]
fn test_an_unterminated_quoted_scalar_at_an_entry_start_still_refuses() {
    assert_eq!(reads("[x, 'tis]"), None);
}

#[test]
fn test_a_quoted_scalar_followed_by_plain_content_still_refuses() {
    assert_eq!(reads("['don't', x]"), None);
}
