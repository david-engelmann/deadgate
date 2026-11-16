//! The YAML document model.
//!
//! A YAML document is an ordered mapping of keys to values where
//! the keys are scalars and equality between keys is the Python
//! equality, so the boolean true and the integer one and the
//! float one are the one key, and two timestamps are the one key
//! only when they are the same kind standing at the same instant.
//! A repeated key keeps its first position and takes the last
//! value, as the Python reading leaves it. Integer tokens that
//! do not fit in a signed 64 bit integer keep their exact digit
//! text, as on the JSON side.

/// A parsed YAML value.
#[derive(Debug, Clone)]
pub enum YamlValue {
    Null,
    Bool(bool),
    Int(i64),
    BigInt(String),
    Float(f64),
    Text(String),
    /// A calendar date standing alone, at its day number.
    Date(i64),
    /// A date with a time, naive, at its instant in microseconds.
    NaiveDateTime(i128),
    /// A date with a time and an offset, at its instant in
    /// microseconds with the offset it carried in seconds.
    AwareDateTime(i128, i64),
    List(Vec<YamlValue>),
    Mapping(Vec<(YamlKey, YamlValue)>),
    /// Bytes standing under the binary tag, decoded from the
    /// scalar's base64 text.
    Bytes(Vec<u8>),
    /// A set standing under the set tag, its members standing
    /// as keys in the order the node's pairs first wrote them.
    Set(Vec<YamlKey>),
    /// A list of pairs standing under the ordered map or the
    /// pairs tag, in the order of the sequence it read from.
    Pairs(Vec<(YamlKey, YamlValue)>),
}

impl YamlValue {
    pub fn as_mapping(&self) -> Option<&Vec<(YamlKey, YamlValue)>> {
        match self {
            YamlValue::Mapping(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<YamlValue>> {
        match self {
            YamlValue::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_mapping(&self) -> bool {
        matches!(self, YamlValue::Mapping(_))
    }

    /// The value standing under a text key, looked up the way the
    /// Python dictionary lookup looks it up.
    pub fn get_text(&self, key: &str) -> Option<&YamlValue> {
        self.get_key(&YamlKey::Text(key.to_string()))
    }

    /// The value standing under a key, looked up the way the
    /// Python dictionary lookup looks it up.
    pub fn get_key(&self, key: &YamlKey) -> Option<&YamlValue> {
        self.as_mapping()?.iter().find_map(
            |(standing, value)| {
                if standing == key {
                    Some(value)
                } else {
                    None
                }
            },
        )
    }

    /// Whether the value is a text equal to the given word.
    pub fn is_text(&self, word: &str) -> bool {
        matches!(self, YamlValue::Text(text) if text == word)
    }
}

/// A key. A mapping's keys and a set's members stand as the
/// scalar kinds, the runtime building both from keys it can
/// hash, while a pair's key stands as any value the document
/// builds, a list of pairs building neither a mapping nor a
/// set. The reader carries the one key model for all three
/// standings, and the border between them is held where the
/// document stands built, in the reader.
#[derive(Debug, Clone)]
pub enum YamlKey {
    Null,
    Bool(bool),
    Int(i64),
    BigInt(String),
    Float(f64),
    Text(String),
    Date(i64),
    NaiveDateTime(i128),
    AwareDateTime(i128, i64),
    Bytes(Vec<u8>),
    List(Vec<YamlValue>),
    Mapping(Vec<(YamlKey, YamlValue)>),
    Set(Vec<YamlKey>),
    Pairs(Vec<(YamlKey, YamlValue)>),
}

impl YamlKey {
    /// The key a value stands as. Every value stands as a
    /// key of its own kind, a container as the container key
    /// a pair may carry; whether the key may stand where it
    /// was read is the reader's border to hold, not this
    /// conversion's.
    pub fn of(value: &YamlValue) -> Option<YamlKey> {
        match value {
            YamlValue::Null => Some(YamlKey::Null),
            YamlValue::Bool(flag) => Some(YamlKey::Bool(*flag)),
            YamlValue::Int(integer) => Some(YamlKey::Int(*integer)),
            YamlValue::BigInt(text) => Some(YamlKey::BigInt(text.clone())),
            YamlValue::Float(number) => Some(YamlKey::Float(*number)),
            YamlValue::Text(text) => Some(YamlKey::Text(text.clone())),
            YamlValue::Date(day) => Some(YamlKey::Date(*day)),
            YamlValue::NaiveDateTime(instant) => Some(YamlKey::NaiveDateTime(*instant)),
            YamlValue::AwareDateTime(instant, offset) => {
                Some(YamlKey::AwareDateTime(*instant, *offset))
            }
            YamlValue::Bytes(bytes) => Some(YamlKey::Bytes(bytes.clone())),
            YamlValue::List(items) => Some(YamlKey::List(items.clone())),
            YamlValue::Mapping(pairs) => Some(YamlKey::Mapping(pairs.clone())),
            YamlValue::Set(members) => Some(YamlKey::Set(members.clone())),
            YamlValue::Pairs(pairs) => Some(YamlKey::Pairs(pairs.clone())),
        }
    }
}

fn float_integer_text(value: f64) -> Option<String> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    if value == 0.0 {
        return Some("0".to_string());
    }
    Some(format!("{value:.0}"))
}

fn key_integer_text(key: &YamlKey) -> Option<String> {
    match key {
        YamlKey::Int(integer) => Some(integer.to_string()),
        YamlKey::BigInt(text) => Some(text.clone()),
        YamlKey::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_string()),
        _ => None,
    }
}

fn key_is_numeric(key: &YamlKey) -> bool {
    matches!(
        key,
        YamlKey::Int(_) | YamlKey::BigInt(_) | YamlKey::Float(_) | YamlKey::Bool(_)
    )
}

fn numeric_keys_equal(a: &YamlKey, b: &YamlKey) -> bool {
    if let (YamlKey::Float(x), YamlKey::Float(y)) = (a, b) {
        // Two readings of the not a number scalar in one
        // document are the one key in the mapping the runtime
        // reading builds, as the key probe recorded, so the
        // second reading takes the value under the first key
        // rather than standing beside it.
        return x == y || (x.is_nan() && y.is_nan());
    }
    let a_float = match a {
        YamlKey::Float(value) => Some(*value),
        _ => None,
    };
    let b_float = match b {
        YamlKey::Float(value) => Some(*value),
        _ => None,
    };
    match (a_float, b_float) {
        (Some(float), None) => match key_integer_text(b) {
            Some(text) => float_integer_text(float).is_some_and(|exact| exact == text),
            None => false,
        },
        (None, Some(float)) => match key_integer_text(a) {
            Some(text) => float_integer_text(float).is_some_and(|exact| exact == text),
            None => false,
        },
        _ => key_integer_text(a) == key_integer_text(b),
    }
}

/// The value a key stands for, a scalar key as the scalar it
/// is and a container key as the container it carries.
pub fn key_to_value(key: &YamlKey) -> YamlValue {
    match key {
        YamlKey::Null => YamlValue::Null,
        YamlKey::Bool(flag) => YamlValue::Bool(*flag),
        YamlKey::Int(integer) => YamlValue::Int(*integer),
        YamlKey::BigInt(text) => YamlValue::BigInt(text.clone()),
        YamlKey::Float(number) => YamlValue::Float(*number),
        YamlKey::Text(text) => YamlValue::Text(text.clone()),
        YamlKey::Date(day) => YamlValue::Date(*day),
        YamlKey::NaiveDateTime(instant) => YamlValue::NaiveDateTime(*instant),
        YamlKey::AwareDateTime(instant, offset) => YamlValue::AwareDateTime(*instant, *offset),
        YamlKey::Bytes(bytes) => YamlValue::Bytes(bytes.clone()),
        YamlKey::List(items) => YamlValue::List(items.clone()),
        YamlKey::Mapping(pairs) => YamlValue::Mapping(pairs.clone()),
        YamlKey::Set(members) => YamlValue::Set(members.clone()),
        YamlKey::Pairs(pairs) => YamlValue::Pairs(pairs.clone()),
    }
}

/// The equality of two values, as the runtime's equality
/// reads them. Two values standing inside a container key
/// meet here when the reader's own machinery compares the
/// keys that carry them, and the reading follows the
/// runtime's rules: numbers of different kinds compare by
/// their value, a mapping compares without regard to its
/// pairs' order, a sequence compares in order, a set by its
/// membership, and a list of pairs pair by pair.
fn values_equal(a: &YamlValue, b: &YamlValue) -> bool {
    match (a, b) {
        (YamlValue::List(x), YamlValue::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(u, v)| values_equal(u, v))
        }
        (YamlValue::Mapping(x), YamlValue::Mapping(y)) => {
            x.len() == y.len()
                && x.iter().all(|(key, value)| {
                    y.iter()
                        .find(|(other, _)| other == key)
                        .is_some_and(|(_, other_value)| values_equal(value, other_value))
                })
        }
        (YamlValue::Set(x), YamlValue::Set(y)) => {
            x.len() == y.len() && x.iter().all(|member| y.contains(member))
        }
        (YamlValue::Pairs(x), YamlValue::Pairs(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((k1, v1), (k2, v2))| k1 == k2 && values_equal(v1, v2))
        }
        _ => YamlKey::of(a) == YamlKey::of(b),
    }
}

impl PartialEq for YamlKey {
    fn eq(&self, other: &YamlKey) -> bool {
        if key_is_numeric(self) && key_is_numeric(other) {
            return numeric_keys_equal(self, other);
        }
        match (self, other) {
            (YamlKey::Null, YamlKey::Null) => true,
            (YamlKey::Text(a), YamlKey::Text(b)) => a == b,
            (YamlKey::Bytes(a), YamlKey::Bytes(b)) => a == b,
            (YamlKey::Date(a), YamlKey::Date(b)) => a == b,
            (YamlKey::NaiveDateTime(a), YamlKey::NaiveDateTime(b)) => a == b,
            // Two aware timestamps are the one key when they
            // stand at the one instant, as the Python equality
            // reads them, whatever offsets they carry.
            (YamlKey::AwareDateTime(ai, _), YamlKey::AwareDateTime(bi, _)) => ai == bi,
            // A naive datetime and an aware datetime never compare
            // equal in Python, and neither compares to a date.
            (YamlKey::List(_), YamlKey::List(_))
            | (YamlKey::Mapping(_), YamlKey::Mapping(_))
            | (YamlKey::Set(_), YamlKey::Set(_))
            | (YamlKey::Pairs(_), YamlKey::Pairs(_)) => {
                values_equal(&key_to_value(self), &key_to_value(other))
            }
            _ => false,
        }
    }
}

impl Eq for YamlKey {}
