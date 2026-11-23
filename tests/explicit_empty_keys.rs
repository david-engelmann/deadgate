//! Pins for slice PORT23, the plain empty explicit key slice.
//! A plain empty explicit key in flow form parts from the
//! runtime at the spaced spelling and at the unspaced
//! spelling and in a flow sequence, and the fences and
//! borders around the shape stand as PORT23 states them.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT23-1, the spaced spelling refusing in a flow mapping.
#[test]
fn test_port23_a_spaced_empty_explicit_key_refuses_in_a_flow_mapping() {
    assert_eq!(reads("{? : 1}"), None);
    assert_eq!(reads("{? : 1, a: 2}"), None);
    assert_eq!(reads("{a: 1, ? : 2}"), None);
    assert_eq!(reads("{? :}"), None);
    assert_eq!(reads("{? : , a: 2}"), None);
    assert_eq!(reads("{? : 'v'}"), None);
    assert_eq!(reads("{? : [1, 2]}"), None);
    assert_eq!(reads("{? : {b: 1}}"), None);
}

// PORT23-2, the unspaced spelling misreading in a flow mapping.
#[test]
fn test_port23_an_unspaced_empty_explicit_key_reads_the_mark_as_a_plain_key() {
    assert_eq!(reads("{?: 1}"), Some("{'?': 1}".to_string()));
    assert_eq!(reads("{?: 1, a: 2}"), Some("{'?': 1, 'a': 2}".to_string()));
    assert_eq!(reads("{a: 1, ?: 2}"), Some("{'a': 1, '?': 2}".to_string()));
    assert_eq!(reads("{?:}"), Some("{'?': None}".to_string()));
    assert_eq!(
        reads("{?: , a: 2}"),
        Some("{'?': None, 'a': 2}".to_string())
    );
}

// PORT23-3, the sequence seats at both spacings.
#[test]
fn test_port23_an_empty_explicit_key_in_a_flow_sequence_reads_the_mark_as_its_key() {
    assert_eq!(reads("[? : 1]"), Some("[{'?': 1}]".to_string()));
    assert_eq!(reads("[?: 1]"), Some("[{'?': 1}]".to_string()));
    assert_eq!(reads("[[?: 1]]"), Some("[[{'?': 1}]]".to_string()));
    assert_eq!(reads("[? : 1, x]"), Some("[{'?': 1}, 'x']".to_string()));
    assert_eq!(reads("{k: [? : 1]}"), Some("{'k': [{'?': 1}]}".to_string()));
}

// PORT23-4, the fences and the controls around the shape.
#[test]
fn test_port23_the_fences_and_controls_around_the_empty_explicit_key_stand() {
    assert_eq!(reads("{?}"), Some("{None: None}".to_string()));
    assert_eq!(reads("{?, a: 2}"), Some("{None: None, 'a': 2}".to_string()));
    assert_eq!(reads("{? k: 1}"), Some("{'k': 1}".to_string()));
    assert_eq!(reads("{? ~: 1}"), Some("{None: 1}".to_string()));
    assert_eq!(reads("{? '': 1}"), Some("{'': 1}".to_string()));
    assert_eq!(reads("{?\t: 1}"), None);
    assert_eq!(reads("{? \t: 1}"), None);
}

// PORT23-5, the separation and property borders of the shape.
#[test]
fn test_port23_the_separation_and_property_borders_of_the_empty_explicit_key_stand() {
    assert_eq!(reads("{?\n: 1}"), None);
    assert_eq!(reads("{?\u{2028}: 1}"), None);
    assert_eq!(reads("{? # c\n: 1}"), None);
    assert_eq!(reads("{? \n: 1}"), None);
    assert_eq!(reads("{? &a: 1}"), None);
    assert_eq!(reads("{? !!str: 1}"), None);
    assert_eq!(reads("{? &a !!str: 1}"), None);
    assert_eq!(reads("{&a ? : 1}"), None);
}

// PORT23-6, the duplicate standings of the shape.
#[test]
fn test_port23_the_duplicate_empty_explicit_keys_stand() {
    assert_eq!(reads("{?: 1, ?: 2}"), Some("{'?': 2}".to_string()));
    assert_eq!(reads("{? : 1, ? : 2}"), None);
    assert_eq!(reads("{?: 1, ? : 2}"), None);
    assert_eq!(reads("{~: 1, ?: 2}"), Some("{None: 1, '?': 2}".to_string()));
}
