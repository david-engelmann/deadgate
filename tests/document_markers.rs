//! PORT8 pins, document markers in every position.
//!
//! Each pin states the runtime reading's answer for one
//! document of the probe set the contract records, the two
//! overreading families and the borders they keep.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> String {
    match parse_yaml(text) {
        Some(value) => python_repr(&value),
        None => "REFUSED".to_string(),
    }
}

#[test]
fn test_a_pair_on_the_start_marker_line_is_refused() {
    assert_eq!(reads("--- a: 1\n"), "REFUSED");
}

#[test]
fn test_a_key_alone_on_the_start_marker_line_is_refused() {
    assert_eq!(reads("--- a:\n"), "REFUSED");
}

#[test]
fn test_a_pair_behind_an_anchor_on_the_marker_line_is_refused() {
    assert_eq!(reads("--- &a a: 1\n"), "REFUSED");
}

#[test]
fn test_a_pair_behind_a_tag_on_the_marker_line_is_refused() {
    assert_eq!(reads("--- !!str a: 1\n"), "REFUSED");
}

#[test]
fn test_a_quoted_pair_on_the_marker_line_is_refused() {
    assert_eq!(reads("--- \"a\": 1\n"), "REFUSED");
}

#[test]
fn test_an_entry_on_the_start_marker_line_is_refused() {
    assert_eq!(reads("--- - 1\n"), "REFUSED");
    assert_eq!(reads("--- -\n"), "REFUSED");
}

#[test]
fn test_a_scalar_on_the_start_marker_line_still_reads() {
    assert_eq!(reads("--- hello\n"), "'hello'");
    assert_eq!(reads("--- a:b\n"), "'a:b'");
}

#[test]
fn test_a_flow_pair_on_the_start_marker_line_still_reads() {
    assert_eq!(reads("--- [a: 1]\n"), "[{'a': 1}]");
    assert_eq!(reads("--- {a: 1}\n"), "{'a': 1}");
}

#[test]
fn test_an_end_marker_with_no_document_is_refused() {
    assert_eq!(reads("...\n"), "REFUSED");
    assert_eq!(reads("... # only\n"), "REFUSED");
}

#[test]
fn test_an_end_marker_behind_only_comments_is_refused() {
    assert_eq!(reads("# c\n...\n"), "REFUSED");
    assert_eq!(reads("\n...\n"), "REFUSED");
}

#[test]
fn test_an_end_marker_behind_a_start_marker_reads_the_empty_document() {
    assert_eq!(reads("---\n...\n"), "None");
    assert_eq!(reads("--- # x\n...\n"), "None");
}

#[test]
fn test_an_end_marker_behind_a_document_still_ends_it() {
    assert_eq!(reads("a: 1\n...\n"), "{'a': 1}");
    assert_eq!(reads("a: 1\n... # done\n"), "{'a': 1}");
}

#[test]
fn test_a_marker_line_inside_a_block_scalar_is_content() {
    assert_eq!(reads("a: |\n  ...\n  text\n"), "{'a': '...\\ntext\\n'}");
    assert_eq!(reads("a: |\n  ---\n  text\n"), "{'a': '---\\ntext\\n'}");
}

#[test]
fn test_a_second_document_still_refuses() {
    assert_eq!(reads("---\na: 1\n---\nb: 2\n"), "REFUSED");
    assert_eq!(reads("a: 1\n...\njunk line\n"), "REFUSED");
}

#[test]
fn test_the_marker_words_standing_as_values_still_read() {
    assert_eq!(reads("a: ---\n"), "{'a': '---'}");
    assert_eq!(reads("a: ...\n"), "{'a': '...'}");
}
