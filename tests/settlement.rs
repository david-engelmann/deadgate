//! The pins of Rust port slice 16, the settlement slice.
//! Every name PORT16-1 and PORT16-2 and PORT16-3 carry
//! stands here as a test against the reader, each one able
//! to fail on the reading it pins.

use deadgate::yaml::{parse_yaml, python_repr};

/// The reading of a document in its repr form, the form in
/// which these pins compare values.
fn reads(doc: &str) -> Option<String> {
    parse_yaml(doc).map(|value| python_repr(&value))
}

// PORT16-1, the settlement hand over, the scalar node under
// the merge tag merging its pair in a flow sequence.

#[test]
fn test_a_merge_tag_on_other_text_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge foo: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_a_quoted_spelling_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge \"<<\": {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_quoted_other_text_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge \"foo\": {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_a_single_quoted_spelling_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge '<<': {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_an_integer_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge 5: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_an_empty_key_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[!!merge : {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_an_anchored_merge_tag_on_other_text_merges_a_pair_in_a_flow_sequence() {
    assert_eq!(
        reads("[&m !!merge foo: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_beside_a_scalar_entry() {
    assert_eq!(
        reads("[x, !!merge foo: {b: 2}]\n"),
        Some("['x', {'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_beside_a_plain_pair() {
    assert_eq!(
        reads("[!!merge foo: {b: 2}, c: 3]\n"),
        Some("[{'b': 2}, {'c': 3}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_nested_inside_a_flow_mapping() {
    assert_eq!(
        reads("{a: [!!merge foo: {b: 2}]}\n"),
        Some("{'a': [{'b': 2}]}".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_a_list_of_mappings() {
    assert_eq!(
        reads("[!!merge foo: [{b: 2}, {c: 3}]]\n"),
        Some("[{'c': 3, 'b': 2}]".to_string())
    );
}

// PORT16-1, the standings that already merge, standing as
// fences on the widening.

#[test]
fn test_a_merge_tag_on_the_plain_spelling_merges_a_pair_in_a_flow_sequence_as_it_does_today() {
    assert_eq!(
        reads("[!!merge <<: {b: 2}]\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_at_a_block_sequence_entry() {
    assert_eq!(
        reads("- !!merge foo: {b: 2}\n"),
        Some("[{'b': 2}]".to_string())
    );
}

#[test]
fn test_a_merge_tag_on_other_text_merges_at_a_flow_mapping_key() {
    assert_eq!(
        reads("{!!merge foo: {b: 2}}\n"),
        Some("{'b': 2}".to_string())
    );
}

#[test]
fn test_a_merge_pair_with_a_scalar_value_is_refused() {
    assert_eq!(reads("[!!merge foo: bar]\n"), None);
}

// PORT16-2, the borders the widening must not cross, the
// merge tagged scalar refusing in entry position and in
// every value position.

#[test]
fn test_a_merge_tag_on_other_text_standing_as_a_flow_sequence_entry_is_refused() {
    assert_eq!(reads("[!!merge foo]\n"), None);
}

#[test]
fn test_a_merge_tag_on_a_quoted_spelling_standing_as_a_flow_sequence_entry_is_refused() {
    assert_eq!(reads("[!!merge \"<<\"]\n"), None);
}

#[test]
fn test_a_merge_tag_on_the_spelling_standing_as_a_flow_sequence_entry_is_refused() {
    assert_eq!(reads("[!!merge <<]\n"), None);
}

#[test]
fn test_a_merge_tag_on_other_text_standing_as_a_flow_mapping_value_is_refused() {
    assert_eq!(reads("{a: !!merge foo}\n"), None);
}

#[test]
fn test_a_merge_tag_on_other_text_standing_as_a_block_value_is_refused() {
    assert_eq!(reads("k: !!merge foo\n"), None);
}

#[test]
fn test_a_merge_tag_on_the_spelling_standing_as_a_block_value_is_refused() {
    assert_eq!(reads("k: !!merge <<\n"), None);
}

#[test]
fn test_a_merge_tag_on_other_text_standing_as_a_pair_value_in_a_sequence_is_refused() {
    assert_eq!(reads("[a: !!merge foo]\n"), None);
}

#[test]
fn test_a_merge_tag_on_the_spelling_standing_as_a_pair_value_in_a_sequence_is_refused() {
    assert_eq!(reads("[a: !!merge <<]\n"), None);
}

#[test]
fn test_a_merge_tag_on_other_text_standing_as_a_block_sequence_entry_is_refused() {
    assert_eq!(reads("- !!merge foo\n"), None);
}

#[test]
fn test_a_merge_tag_standing_alone_at_a_block_value_is_refused() {
    assert_eq!(reads("k: !!merge\n"), None);
}

#[test]
fn test_a_merge_tag_standing_at_a_block_value_with_a_scalar_behind_it_is_refused() {
    assert_eq!(reads("k: !!merge\n  foo\n"), None);
}

#[test]
fn test_a_merge_tag_on_a_sequence_standing_as_a_pair_key_still_refuses() {
    assert_eq!(reads("[!!merge [a]: {b: 2}]\n"), None);
}

// PORT16-3, the omap and pairs refusal, an entry standing
// as one pair through a merge refusing the reading whole.

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_pair_is_refused() {
    assert_eq!(reads("!!omap\n- !!merge <<: {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_with_a_plain_entry_beside_a_merge_entry_is_refused() {
    assert_eq!(reads("!!omap\n- a: 1\n- !!merge <<: {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_the_untagged_merge_spelling_is_refused() {
    assert_eq!(reads("!!omap\n- <<: {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_flow_entry_carrying_a_merge_pair_is_refused() {
    assert_eq!(reads("!!omap\n- {<<: {b: 2}}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_tag_on_other_text_is_refused() {
    assert_eq!(reads("!!omap\n- !!merge foo: {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_tag_on_a_quoted_spelling_is_refused() {
    assert_eq!(reads("!!omap\n- !!merge \"<<\": {b: 2}\n"), None);
}

#[test]
fn test_a_pairs_entry_carrying_a_merge_pair_is_refused() {
    assert_eq!(reads("!!pairs\n- !!merge <<: {b: 2}\n"), None);
}

#[test]
fn test_a_pairs_with_a_plain_entry_beside_a_merge_entry_is_refused() {
    assert_eq!(reads("!!pairs\n- a: 1\n- !!merge <<: {b: 2}\n"), None);
}

#[test]
fn test_a_pairs_entry_carrying_the_untagged_merge_spelling_is_refused() {
    assert_eq!(reads("!!pairs\n- <<: {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_standing_as_a_merge_pair_in_flow_dress_is_refused() {
    assert_eq!(reads("!!omap [<<: {b: 2}]\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_with_an_empty_source_beside_a_plain_pair_is_refused()
{
    assert_eq!(reads("!!omap\n- {<<: {}, a: 1}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_repeated_keys_is_refused() {
    assert_eq!(reads("!!omap\n- {a: 1, a: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_whose_merge_source_stands_under_an_anchor_is_refused() {
    assert_eq!(reads("!!omap\n- <<: &s {b: 2}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_standing_under_a_map_tag_with_a_merge_pair_is_refused() {
    assert_eq!(reads("!!omap\n- !!map {<<: {b: 2}}\n"), None);
}

// PORT16-3, the borders that already agree, standing as
// fences on the refusal.

#[test]
fn test_an_ordered_map_in_flow_dress_reads_as_pairs() {
    assert_eq!(
        reads("!!omap [{a: 1}, {b: 2}]\n"),
        Some("[('a', 1), ('b', 2)]".to_string())
    );
}

#[test]
fn test_an_ordered_map_with_one_plain_entry_reads_as_pairs() {
    assert_eq!(reads("!!omap\n- {b: 2}\n"), Some("[('b', 2)]".to_string()));
}

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_pair_beside_a_plain_pair_is_refused() {
    assert_eq!(reads("!!omap\n- {<<: {b: 2}, a: 1}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_whose_merge_source_is_a_list_is_refused() {
    assert_eq!(reads("!!omap\n- <<: [{b: 2}, {c: 3}]\n"), None);
}

#[test]
fn test_an_ordered_map_entry_carrying_a_merge_with_an_empty_source_is_refused() {
    assert_eq!(reads("!!omap\n- {<<: {}}\n"), None);
}

#[test]
fn test_an_ordered_map_entry_standing_as_an_alias_to_a_plain_mapping_reads() {
    assert_eq!(
        reads("base: &x {b: 2}\nomap: !!omap\n  - *x\n"),
        Some("{'base': {'b': 2}, 'omap': [('b', 2)]}".to_string())
    );
}

#[test]
fn test_an_ordered_map_entry_standing_as_an_alias_to_a_merge_mapping_reads() {
    assert_eq!(
        reads("base: &x {<<: {b: 2}}\nomap: !!omap\n  - *x\n"),
        Some("{'base': {'b': 2}, 'omap': [('b', 2)]}".to_string())
    );
}

#[test]
fn test_an_ordered_map_entry_carrying_a_mapping_whose_value_merges_reads() {
    assert_eq!(
        reads("!!omap\n- {a: {<<: {b: 2}}}\n"),
        Some("[('a', {'b': 2})]".to_string())
    );
}
