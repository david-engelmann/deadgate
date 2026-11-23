//! Pins for slice PORT22, the break border pair slice.
//! The pins of PORT22-1 to PORT22-5 stood retired by
//! replacement in the PORT25 cure slice, their cells
//! standing as cells of the successor pins in
//! tests/property_readings.rs in their moved standing,
//! and the pin of PORT22-6 stands here as it stood.

use deadgate::yaml::{parse_yaml, python_repr};

fn reads(text: &str) -> Option<String> {
    parse_yaml(text).map(|value| python_repr(&value))
}

// PORT22-6, the borders that part the pair from its family.
#[test]
fn test_port22_the_borders_around_the_pair_stand() {
    assert_eq!(reads("[&a : 1]"), Some("[{None: 1}]".to_string()));
    assert_eq!(reads("[&a\n, x]"), Some("[None, 'x']".to_string()));
    assert_eq!(reads("[&a\n]"), Some("[None]".to_string()));
    assert_eq!(reads("{&a\n: 1}"), None);
    assert_eq!(reads("{&a\u{2028}: 1}"), None);
    assert_eq!(reads("{[&a\n: 1]: v}"), None);
    assert_eq!(reads("[&a\t: 1]"), None);
}
