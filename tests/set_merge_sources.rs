//! Pins for slice PORT25, the set standing as a merge
//! source. A set standing as a merge source merges into
//! the carrying mapping as its members standing as keys
//! with null values, at a direct merge key and behind an
//! alias merge key alike, and the borders and neighbours
//! of the shape stand as PORT25 states them.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT25-1, the direct core.
#[test]
fn test_port25_a_set_standing_as_a_merge_source_merges_its_members_as_keys_with_null_values_at_a_direct_merge_key(
) {
    assert_eq!(
        reads("{<<: !!set {a, b}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("<<: !!set {a, b}\nc: 1\n"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("<<: !!set\n  ? a\n  ? b\nc: 1\n"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {a, b}}"),
        Some("{'a': None, 'b': None}".to_string())
    );
    assert_eq!(
        reads("{!!merge <<: !!set {a, b}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: &s !!set {a, b}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{d: &s !!set {a, b}, m: {<<: *s, c: 1}}"),
        Some("{'d': {'a', 'b'}, 'm': {'a': None, 'b': None, 'c': 1}}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {'a', 'b'}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {1, 2}, c: 1}"),
        Some("{1: None, 2: None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {? a, ? b}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {a}, c: 1}"),
        Some("{'a': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {true, false}, c: 1}"),
        Some("{True: None, False: None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {~, a}, c: 1}"),
        Some("{None: None, 'a': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {a: ~, b: ~}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
}

// PORT25-2, the alias merge key seat.
#[test]
fn test_port25_a_set_standing_as_the_pair_value_behind_an_alias_merge_key_merges() {
    assert_eq!(
        reads("&m <<: {z: 0}\n? *m\n: !!set {a, b}\n"),
        Some("{'z': 0, 'a': None, 'b': None}".to_string())
    );
    assert_eq!(
        reads("&m <<: {z: 0}\n*m: !!set {a, b}\n"),
        Some("{'z': 0, 'a': None, 'b': None}".to_string())
    );
    assert_eq!(
        reads("x: {&m <<: {z: 0}}\ny: {*m: !!set {a, b}}\n"),
        Some("{'x': {'z': 0}, 'y': {'a': None, 'b': None}}".to_string())
    );
}

// PORT25-3, the seats and the list shapes.
#[test]
fn test_port25_a_set_standing_in_a_list_of_merge_sources_merges_at_its_seats_and_in_its_order() {
    assert_eq!(
        reads("{m: {<<: !!set {a, b}, c: 1}}"),
        Some("{'m': {'a': None, 'b': None, 'c': 1}}".to_string())
    );
    assert_eq!(
        reads("[<<: !!set {a, b}]"),
        Some("[{'a': None, 'b': None}]".to_string())
    );
    assert_eq!(
        reads("[<<: !!set {a, b}, x]"),
        Some("[{'a': None, 'b': None}, 'x']".to_string())
    );
    assert_eq!(
        reads("{<<: [!!set {a, b}], c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: [!!set {a, b}, {d: 2}], c: 1}"),
        Some("{'d': 2, 'a': None, 'b': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: [{d: 2}, !!set {a, b}], c: 1}"),
        Some("{'a': None, 'b': None, 'd': 2, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: [!!set {a}, !!set {b}], c: 1}"),
        Some("{'b': None, 'a': None, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {a}, <<: {b: 2}, c: 1}"),
        Some("{'a': None, 'b': 2, 'c': 1}".to_string())
    );
}

// PORT25-4, the ordering and the override.
#[test]
fn test_port25_the_merged_members_of_a_set_source_stand_in_front_and_explicit_pairs_override_them()
{
    assert_eq!(
        reads("{<<: !!set {a, b}, a: 9}"),
        Some("{'a': 9, 'b': None}".to_string())
    );
    assert_eq!(
        reads("{a: 9, <<: !!set {a, b}}"),
        Some("{'a': 9, 'b': None}".to_string())
    );
    assert_eq!(
        reads("{d: &s !!set {x}, <<: !!set {a, b}, q: {<<: *s}}"),
        Some("{'a': None, 'b': None, 'd': {'x'}, 'q': {'x': None}}".to_string())
    );
    assert_eq!(
        reads("{<<: !!set {a, a, b}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
}

// PORT25-5, the borders the cure keeps.
#[test]
fn test_port25_the_borders_of_a_set_standing_as_a_merge_source_stand() {
    assert_eq!(reads("{<<: !!set {}, c: 1}"), Some("{'c': 1}".to_string()));
    assert_eq!(reads("{<<: !!set {{a: 1}, b}, c: 1}"), None);
    assert_eq!(reads("{<<: !!set {[1], b}, c: 1}"), None);
    assert_eq!(reads("{<<: !!set {a, !!set {b}}, c: 1}"), None);
    assert_eq!(reads("{<<: 5, c: 1}"), None);
    assert_eq!(
        reads("{<<: !!set {a: 1, b: 2}, c: 1}"),
        Some("{'a': None, 'b': None, 'c': 1}".to_string())
    );
}

// PORT25-6, the neighbours standing unmoved.
#[test]
fn test_port25_the_neighbours_of_a_set_standing_as_a_merge_source_stand_unmoved() {
    assert_eq!(reads("{<<: !!omap [a: 1, b: 2], c: 1}"), None);
    assert_eq!(reads("{<<: !!pairs [a: 1, b: 2], c: 1}"), None);
    assert_eq!(
        reads("{<<: {a: 1}, c: 1}"),
        Some("{'a': 1, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{<<: [{a: 1}, {b: 2}], c: 1}"),
        Some("{'b': 2, 'a': 1, 'c': 1}".to_string())
    );
    assert_eq!(
        reads("{x: !!set {a, b}}"),
        Some("{'x': {'a', 'b'}}".to_string())
    );
    assert_eq!(reads("a: &s !!set {x, y}\n? *s\n: v\n"), None);
}
