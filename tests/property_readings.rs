//! The pins of Rust port slice 19, the property readings slice.
//! Every name PORT19-1 to PORT19-15 carry stands here as
//! a test against the reader, each one able to fail on
//! the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT19-2, the key anchor end set and its ground.

#[test]
fn test_port19_a_flow_key_whose_anchor_ends_at_a_separator_refuses() {
    assert_eq!(reads("{&a\u{2028}k: 1}"), None);
    assert_eq!(reads("{&a\u{2029}k: 1}"), None);
}

#[test]
fn test_port19_a_flow_key_whose_anchor_ends_at_a_line_feed_reads() {
    assert_eq!(reads("{&a\nk: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{&a #c\nk: 1}"), Some("{'k': 1}".to_string()));
}

// PORT19-3, the verbatim after set at its seats.

#[test]
fn test_port19_a_flow_key_whose_verbatim_tag_ends_at_a_separator_refuses_and_at_a_line_feed_reads()
{
    assert_eq!(reads("{!<tag:yaml.org,2002:str>\u{2028}k: 1}"), None);
    assert_eq!(reads("{!<tag:yaml.org,2002:str>\u{2029}k: 1}"), None);
    assert_eq!(
        reads("{!<tag:yaml.org,2002:str>\nk: 1}"),
        Some("{'k': 1}".to_string())
    );
}

#[test]
fn test_port19_an_explicit_key_whose_property_ends_at_a_separator_refuses() {
    assert_eq!(reads("{? &a\u{2028}k: 1}"), None);
    assert_eq!(reads("{? !<tag:yaml.org,2002:str>\u{2028}k: 1}"), None);
}

#[test]
fn test_port19_a_verbatim_tag_ending_at_a_separator_refuses_and_at_a_line_feed_reads_in_value_position(
) {
    assert_eq!(reads("[!<tag:yaml.org,2002:str>\u{2028}x]"), None);
    assert_eq!(reads("[!<tag:yaml.org,2002:str>\u{2029}x]"), None);
    assert_eq!(
        reads("[!<tag:yaml.org,2002:str>\nx]"),
        Some("['x']".to_string())
    );
}

// PORT19-4, the shorthand token at the key seat and the value seat.

#[test]
fn test_port19_a_flow_key_whose_shorthand_tag_ends_at_a_break_reads() {
    assert_eq!(reads("{!!str\u{2028}k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{!!str\u{2029}k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{!!str\nk: 1}"), Some("{'k': 1}".to_string()));
}

#[test]
fn test_port19_a_shorthand_tag_ending_at_a_separator_reads_the_entry_behind_it_in_a_mapping_value()
{
    assert_eq!(reads("{k: !!str\u{2028}v}"), Some("{'k': 'v'}".to_string()));
}

// PORT19-5, the empty node property shapes in value position.

#[test]
fn test_port19_an_anchor_standing_alone_reads_as_the_empty_entry() {
    assert_eq!(reads("[&a]"), Some("[None]".to_string()));
    assert_eq!(reads("{k: &a}"), Some("{'k': None}".to_string()));
    assert_eq!(reads("[&a, &b]"), Some("[None, None]".to_string()));
}

#[test]
fn test_port19_a_tag_standing_alone_converts_the_empty_entry() {
    assert_eq!(reads("[!!str]"), Some("['']".to_string()));
    assert_eq!(reads("[!!null]"), Some("[None]".to_string()));
    assert_eq!(reads("[!!binary]"), Some("[b'']".to_string()));
}

#[test]
fn test_port19_a_tag_standing_alone_refuses_where_the_empty_conversion_does_not_stand() {
    assert_eq!(reads("[!!int]"), None);
    assert_eq!(reads("[!!float]"), None);
    assert_eq!(reads("[!!bool]"), None);
}

#[test]
fn test_port19_the_non_specific_tag_in_flow_position() {
    assert_eq!(reads("[! x]"), Some("['x']".to_string()));
    assert_eq!(reads("{! k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("[! [1]]"), Some("[[1]]".to_string()));
    assert_eq!(reads("{! [1]: 2}"), None);
}

// PORT19-6, the empty shapes at the key seat.

#[test]
fn test_port19_an_anchor_standing_alone_as_a_flow_key_refuses() {
    assert_eq!(reads("{&a: 1}"), None);
    assert_eq!(reads("{&a, k: 1}"), None);
    assert_eq!(reads("{&a}"), None);
}

#[test]
fn test_port19_a_tag_standing_alone_as_a_flow_key_refuses() {
    assert_eq!(reads("{!!str: 1}"), None);
    assert_eq!(reads("{!!str}"), None);
}

#[test]
fn test_port19_an_anchored_empty_explicit_key_reads_at_the_comma_and_the_brace_and_refuses_at_the_colon(
) {
    assert_eq!(
        reads("{? &a, k: 1}"),
        Some("{None: None, 'k': 1}".to_string())
    );
    assert_eq!(reads("{? &a}"), Some("{None: None}".to_string()));
    assert_eq!(reads("{? &a: 1}"), None);
}

// PORT19-7, the duplicate and order standings in flow position.

#[test]
fn test_port19_the_explicit_takings_stand_together() {
    assert_eq!(reads("{&a ? !!str k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{!!str ? &a k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{&a ? &b k: 1}"), None);
    assert_eq!(reads("{!!str ? !!int k: 1}"), None);
}

#[test]
fn test_port19_an_anchor_and_a_tag_in_either_order_read() {
    assert_eq!(reads("[&a !!str x]"), Some("['x']".to_string()));
    assert_eq!(reads("[!!str &a x]"), Some("['x']".to_string()));
    assert_eq!(reads("{&a !!str k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{!!str &a k: 1}"), Some("{'k': 1}".to_string()));
}

#[test]
fn test_port19_a_second_property_of_a_kind_in_one_taking_refuses() {
    assert_eq!(reads("[&a &b x]"), None);
    assert_eq!(reads("[!!int !!str x]"), None);
    assert_eq!(reads("{&a &b k: 1}"), None);
    assert_eq!(reads("{!!int !!str k: 1}"), None);
}

#[test]
fn test_port19_an_anchor_name_rebound_across_entries_refuses() {
    assert_eq!(reads("[&a x, &a y]"), None);
    assert_eq!(reads("{k: &a 1, j: &a 2}"), None);
}

// PORT19-8, a property standing in front of an alias.

#[test]
fn test_port19_a_property_standing_in_front_of_an_alias_refuses() {
    assert_eq!(reads("[&a x, &b *a]"), None);
    assert_eq!(reads("{&a k: 1, &b *a: 2}"), None);
    assert_eq!(reads("{&a k: 1, !!str *a: 2}"), None);
}

// PORT19-9, the no separation borders.

#[test]
fn test_port19_a_property_standing_against_a_collection_opener_refuses() {
    assert_eq!(reads("[&a[1]]"), None);
    assert_eq!(reads("[!!seq[1]]"), None);
    assert_eq!(reads("[!<tag:yaml.org,2002:seq>[1]]"), None);
}

#[test]
fn test_port19_a_property_standing_against_a_quote_or_a_second_property_refuses() {
    assert_eq!(reads("[&a\"x\"]"), None);
    assert_eq!(reads("[!!str\"x\"]"), None);
    assert_eq!(reads("[&a!!str x]"), None);
    assert_eq!(reads("[!!str&a x]"), None);
}

// PORT19-10, the anchor grammar in flow position.

#[test]
fn test_port19_the_anchor_name_grammar_in_flow_position() {
    assert_eq!(reads("[&a.b x]"), None);
    assert_eq!(reads("{&a.b k: 1}"), None);
    assert_eq!(reads("[& x]"), None);
    assert_eq!(reads("[&a-b_c9 x]"), Some("['x']".to_string()));
}

// PORT19-11, the verbatim borders.

#[test]
fn test_port19_the_verbatim_borders() {
    assert_eq!(reads("[!<tag:yaml.org,2002:str\u{2028}> x]"), None);
    assert_eq!(reads("[!<> x]"), None);
    assert_eq!(reads("[!<tag:yaml.org,2002:str x]"), None);
    assert_eq!(reads("[!<tag:yaml.org,2002:str>x]"), None);
}

// PORT19-12, the key tag classes in flow position.

#[test]
fn test_port19_a_converting_tag_on_a_flow_key_converts() {
    assert_eq!(reads("{!!int 0o17: a}"), Some("{15: 'a'}".to_string()));
    assert_eq!(
        reads("{!!binary aGk=: a}"),
        Some("{b'hi': 'a'}".to_string())
    );
}

#[test]
fn test_port19_the_refusing_key_tag_classes_refuse() {
    assert_eq!(reads("{!!int a: 1}"), None);
    assert_eq!(reads("{!!merge k: 1}"), None);
    assert_eq!(reads("{!!set k: 1}"), None);
    assert_eq!(reads("{!!seq k: 1}"), None);
    assert_eq!(reads("{!!omap k: 1}"), None);
}

#[test]
fn test_port19_the_merge_spelling_and_its_tagged_form_read_as_merge_keys() {
    assert_eq!(reads("{<<: {a: 1}}"), Some("{'a': 1}".to_string()));
    assert_eq!(reads("{!!merge <<: {a: 1}}"), Some("{'a': 1}".to_string()));
}

// PORT19-13, the text end asymmetry.

#[test]
fn test_port19_a_text_ending_behind_a_property_refuses() {
    assert_eq!(reads("{&a"), None);
    assert_eq!(reads("{&a "), None);
    assert_eq!(reads("[&a"), None);
}

// PORT19-14, the tab standing at the property seats.

#[test]
fn test_port19_a_tab_standing_at_a_property_border_refuses_at_the_door() {
    assert_eq!(reads("[&a\tx]"), None);
    assert_eq!(reads("[!!str\tx]"), None);
    assert_eq!(reads("[x,\t&a y]"), None);
}

// PORT19-15, the pair taking break border at the anchored empty entry.

#[test]
fn test_port19_an_anchored_empty_pair_key_with_a_break_behind_its_anchor_reads() {
    assert_eq!(reads("[&a\u{2028}: 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n: 1]"), Some("[{None: 1}]".to_string()));
}
