//! The pins of Rust port slice 20, the property split refusal
//! frame slice. Every pin name PORT20-1 to PORT20-5 carry
//! stands here as a test against the reader, each one able
//! to fail on the reading it pins. PORT20-6 carries no pin
//! of its own, its fences standing re run at the split seat
//! in the drafting of the slice.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT20-1, the refusal shapes of the shared split.

#[test]
fn test_port20_a_second_anchor_refuses() {
    assert_eq!(reads("&a &b k: v\n"), None);
}

#[test]
fn test_port20_a_second_anchor_behind_a_tag_refuses() {
    assert_eq!(reads("!!str &a &b k: v\n"), None);
}

#[test]
fn test_port20_a_second_tag_refuses() {
    assert_eq!(reads("!!str !!int k: v\n"), None);
}

#[test]
fn test_port20_an_empty_anchor_name_refuses() {
    assert_eq!(reads("& k: v\n"), None);
}

#[test]
fn test_port20_an_anchor_ending_at_a_comma_refuses() {
    assert_eq!(reads("&a, k: v\n"), None);
}

#[test]
fn test_port20_an_anchor_ending_at_a_colon_refuses() {
    assert_eq!(reads("&a: k: v\n"), None);
}

#[test]
fn test_port20_an_unterminated_verbatim_tag_refuses() {
    assert_eq!(reads("!<tag:x k: v\n"), None);
}

#[test]
fn test_port20_an_empty_verbatim_tag_refuses() {
    assert_eq!(reads("!<> k: v\n"), None);
}

#[test]
fn test_port20_a_verbatim_tag_with_a_letter_behind_it_refuses() {
    assert_eq!(reads("!<tag:x>z k: v\n"), None);
}

#[test]
fn test_port20_a_tag_with_an_unbound_handle_refuses() {
    assert_eq!(reads("!e!str k: v\n"), None);
}

// PORT20-2, every caller refusing whole on the refusal arm.

#[test]
fn test_port20_a_refusing_value_rest_refuses() {
    assert_eq!(reads("k: &a &b v\n"), None);
}

#[test]
fn test_port20_a_refusing_pair_line_refuses() {
    assert_eq!(reads("a: 1\n&x &y b: 2\n"), None);
}

#[test]
fn test_port20_a_refusing_dash_content_refuses() {
    assert_eq!(reads("- &a &b v\n"), None);
}

#[test]
fn test_port20_a_refusing_explicit_key_refuses() {
    assert_eq!(reads("? &a &b k\n: v\n"), None);
}

#[test]
fn test_port20_a_refusing_nested_pair_line_refuses() {
    assert_eq!(reads("outer:\n  &a &b k: v\n"), None);
}

#[test]
fn test_port20_a_refusing_properties_alone_line_refuses() {
    assert_eq!(reads("&a &b\nk: v\n"), None);
}

#[test]
fn test_port20_a_refusing_colon_line_value_refuses() {
    assert_eq!(reads("? k\n: &a &b v\n"), None);
}

#[test]
fn test_port20_a_refusing_dash_content_in_a_mapping_value_refuses() {
    assert_eq!(reads("k:\n  - &a &b v\n"), None);
}

// PORT20-3, the key split its classes and the carried standings.

#[test]
fn test_port20_an_unknown_spelling_in_key_position_refuses_at_the_key_seat() {
    assert_eq!(reads("!!foo k: v\n"), None);
}

#[test]
fn test_port20_an_unknown_spelling_in_value_position_refuses_at_the_finish() {
    assert_eq!(reads("k: !!foo v\n"), None);
}

#[test]
fn test_port20_a_collection_tag_in_key_position_refuses_at_the_key_seat() {
    assert_eq!(reads("!!seq k: v\n"), None);
}

#[test]
fn test_port20_a_collection_tag_at_an_explicit_key_refuses_at_the_key_seat() {
    assert_eq!(reads("? !!seq k\n: v\n"), None);
}

#[test]
fn test_port20_a_comma_joined_spelling_in_key_position_refuses() {
    assert_eq!(reads("!!str, k: v\n"), None);
}

#[test]
fn test_port20_a_comma_joined_spelling_in_value_position_refuses() {
    assert_eq!(reads("k: !!str, v\n"), None);
}

#[test]
fn test_port20_a_verbatim_unknown_spelling_in_key_position_refuses() {
    assert_eq!(reads("!<tag:x> k: v\n"), None);
}

// PORT20-4, the standings that agree beside the frame.

#[test]
fn test_port20_an_anchor_in_front_of_a_key_reads_and_binds_the_key() {
    assert_eq!(
        reads("&a k: v\nb: *a\n"),
        Some("{'k': 'v', 'b': 'k'}".to_string())
    );
}

#[test]
fn test_port20_a_verbatim_standard_spelling_reads() {
    assert_eq!(
        reads("!<tag:yaml.org,2002:str> k: v\n"),
        Some("{'k': 'v'}".to_string())
    );
}

#[test]
fn test_port20_a_bound_handle_spelling_reads() {
    assert_eq!(
        reads("%TAG !e! tag:yaml.org,2002:\n---\n!e!str k: v\n"),
        Some("{'k': 'v'}".to_string())
    );
}

#[test]
fn test_port20_a_non_specific_tag_in_key_position_reads() {
    assert_eq!(reads("! k: v\n"), Some("{'k': 'v'}".to_string()));
}

#[test]
fn test_port20_properties_alone_bind_the_following_block() {
    assert_eq!(reads("&a\nk: v\n"), Some("{'k': 'v'}".to_string()));
}

#[test]
fn test_port20_two_spaces_behind_an_anchor_read() {
    assert_eq!(reads("&a  k: v\n"), Some("{'k': 'v'}".to_string()));
}

#[test]
fn test_port20_an_anchor_and_a_tag_in_front_of_a_key_read() {
    assert_eq!(reads("&a !!str k: v\n"), Some("{'k': 'v'}".to_string()));
}

#[test]
fn test_port20_properties_alone_on_a_dash_line_bind_the_following_block() {
    assert_eq!(reads("- &a\n  k: v\n"), Some("[{'k': 'v'}]".to_string()));
}

#[test]
fn test_port20_a_tag_alone_on_a_dash_line_settles_on_a_following_scalar() {
    assert_eq!(reads("- !!str\n  word\n"), Some("['word']".to_string()));
}

#[test]
fn test_port20_a_constructor_tag_alone_on_a_dash_line_refuses_on_a_following_mapping() {
    assert_eq!(reads("- !!str\n  k: v\n"), None);
}

// PORT20-5, the pair shapes and the gathering folds.

#[test]
fn test_port20_a_refusing_line_folds_into_an_explicit_key() {
    assert_eq!(
        reads("? a\u{2028} &x &y b: 2\n: 1\n"),
        Some("{'a\\u2028&x &y b: 2': 1}".to_string())
    );
}

#[test]
fn test_port20_a_refusing_line_folds_into_a_plain_value() {
    assert_eq!(
        reads("k: word\n  &x &y b\n"),
        Some("{'k': 'word &x &y b'}".to_string())
    );
}

#[test]
fn test_port20_a_colon_space_segment_refuses_in_a_plain_value() {
    assert_eq!(reads("k: word\n  &x &y b: 2\n"), None);
}

#[test]
fn test_port20_a_standing_pair_line_stops_the_gathering() {
    assert_eq!(reads("? a\u{2028} &x b: 2\n: 1\n"), None);
}
