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

/// A mapping key. Keys are scalars only in the documents this
/// tool reads.
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
}

impl YamlKey {
    /// The key a scalar value stands as, or nothing when the
    /// value is a container and so cannot be a key here.
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
            YamlValue::List(_) | YamlValue::Mapping(_) => None,
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

/// The value a mapping key stands for, the key being a scalar
/// by construction.
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
            (YamlKey::Date(a), YamlKey::Date(b)) => a == b,
            (YamlKey::NaiveDateTime(a), YamlKey::NaiveDateTime(b)) => a == b,
            // Two aware timestamps are the one key when they
            // stand at the one instant, as the Python equality
            // reads them, whatever offsets they carry.
            (YamlKey::AwareDateTime(ai, _), YamlKey::AwareDateTime(bi, _)) => ai == bi,
            // A naive datetime and an aware datetime never compare
            // equal in Python, and neither compares to a date.
            _ => false,
        }
    }
}

impl Eq for YamlKey {}
