//! The YAML reading, mirroring the PyYAML safe load of the
//! runtime this repo runs on, and the Python text forms of the
//! values it produces.
//!
//! The reading covers block mappings and sequences, flow
//! collections, plain scalars with their continuation folding,
//! and quoted scalars standing on one line, and it resolves
//! plain scalars with the resolver of the runtime, the YAML 1.1
//! spellings standing as the probe recorded them. The limits the
//! contract states stand at the edges, document apparatus
//! beyond one leading opener and one trailing closer refused,
//! block scalars refused in flow position and read in block
//! position, quoted scalars spanning lines read under the
//! seventh slice inside its borders, a mapping
//! key that is a container refused, and nesting bounded at
//! 4000. Anchors and aliases and merge keys and tags stand
//! read under the fourth slice inside its borders, properties
//! standing in key position read under the sixth slice inside
//! its borders, a tag outside the standing set refused,
//! and alias and merge expansion bounded at 4000 across one
//! document. A scalar
//! the resolver reads as a date or a timestamp whose value does
//! not stand refuses the document, as the Python command dies on
//! it with an uncaught error.

use std::collections::{HashMap, HashSet};

use crate::dates::{civil_from_days, days_from_civil, valid_calendar_date};
use crate::yamlvalue::{key_to_value, YamlKey, YamlValue};

/// The deepest nesting the reader accepts.
pub const MAX_YAML_NESTING: usize = 4000;

// ----------------------------------------------------------------
// Python text forms
// ----------------------------------------------------------------

/// The shortest digit spelling of a float, laid out the way the
/// Python repr of a float lays it out, fixed form while the
/// decimal point stands between three places before the first
/// digit and sixteen after the start, scientific outside.
pub fn python_float_text(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        };
    }
    let negative = value < 0.0;
    let spelled = format!("{:e}", value.abs());
    let (mantissa, exponent_text) = spelled.split_once('e').expect("exponent form");
    let exponent: i64 = exponent_text.parse().expect("exponent digits");
    let digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
    let decpt = exponent + 1;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if (-3..=16).contains(&decpt) {
        if decpt <= 0 {
            out.push_str("0.");
            for _ in 0..(-decpt) {
                out.push('0');
            }
            out.push_str(&digits);
        } else if (decpt as usize) >= digits.len() {
            out.push_str(&digits);
            for _ in 0..(decpt as usize - digits.len()) {
                out.push('0');
            }
            out.push_str(".0");
        } else {
            out.push_str(&digits[..decpt as usize]);
            out.push('.');
            out.push_str(&digits[decpt as usize..]);
        }
    } else {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let sci = decpt - 1;
        if sci < 0 {
            out.push('-');
        } else {
            out.push('+');
        }
        let magnitude = sci.abs();
        if magnitude < 10 {
            out.push('0');
        }
        out.push_str(&magnitude.to_string());
    }
    out
}

fn date_text(day_number: i64) -> String {
    let (year, month, day) = civil_from_days(day_number);
    format!("{year:04}-{month:02}-{day:02}")
}

fn datetime_parts(instant: i128) -> (i64, i64, i64, i64, i64, i64, i64) {
    let day_number = crate::dates::utc_day_of_instant(instant);
    let (year, month, day) = civil_from_days(day_number);
    let within = instant - day_number as i128 * 86_400_000_000;
    let seconds = (within / 1_000_000) as i64;
    let micros = (within % 1_000_000) as i64;
    (
        year,
        month,
        day,
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60,
        micros,
    )
}

fn offset_suffix(offset_seconds: i64) -> String {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let magnitude = offset_seconds.abs();
    let hours = magnitude / 3600;
    let minutes = (magnitude % 3600) / 60;
    let seconds = magnitude % 60;
    if seconds != 0 {
        format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{sign}{hours:02}:{minutes:02}")
    }
}

/// The Python str of a YAML value, as the reconciliation reads
/// it when a name or a job value is asked for as a string.
pub fn python_str(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "None".to_string(),
        YamlValue::Bool(flag) => {
            if *flag {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        YamlValue::Int(integer) => integer.to_string(),
        YamlValue::BigInt(text) => text.clone(),
        YamlValue::Float(number) => python_float_text(*number),
        YamlValue::Text(text) => text.clone(),
        YamlValue::Date(day) => date_text(*day),
        YamlValue::NaiveDateTime(instant) => {
            let (year, month, day, hour, minute, second, micros) = datetime_parts(*instant);
            let base = format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}");
            if micros != 0 {
                format!("{base}.{micros:06}")
            } else {
                base
            }
        }
        YamlValue::AwareDateTime(instant, offset) => {
            let local = instant + *offset as i128 * 1_000_000;
            let (year, month, day, hour, minute, second, micros) = datetime_parts(local);
            let base = format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}");
            let with_fraction = if micros != 0 {
                format!("{base}.{micros:06}")
            } else {
                base
            };
            format!("{with_fraction}{}", offset_suffix(*offset))
        }
        YamlValue::List(_)
        | YamlValue::Mapping(_)
        | YamlValue::Bytes(_)
        | YamlValue::Set(_)
        | YamlValue::Pairs(_) => python_repr(value),
    }
}

fn timedelta_repr(total_seconds: i64) -> String {
    let days = total_seconds.div_euclid(86_400);
    let seconds = total_seconds.rem_euclid(86_400);
    let mut parts = Vec::new();
    if days != 0 {
        parts.push(format!("days={days}"));
    }
    if seconds != 0 || parts.is_empty() {
        parts.push(format!("seconds={seconds}"));
    }
    format!("datetime.timedelta({})", parts.join(", "))
}

fn datetime_repr(instant: i128, offset: Option<i64>) -> String {
    let local = instant + offset.unwrap_or(0) as i128 * 1_000_000;
    let (year, month, day, hour, minute, second, micros) = datetime_parts(local);
    let mut parts = vec![
        year.to_string(),
        month.to_string(),
        day.to_string(),
        hour.to_string(),
        minute.to_string(),
    ];
    if second != 0 || micros != 0 {
        parts.push(second.to_string());
    }
    if micros != 0 {
        parts.push(micros.to_string());
    }
    let mut out = format!("datetime.datetime({})", parts.join(", "));
    if let Some(offset_seconds) = offset {
        out.truncate(out.len() - 1);
        if offset_seconds == 0 {
            out.push_str(", tzinfo=datetime.timezone.utc)");
        } else {
            out.push_str(&format!(
                ", tzinfo=datetime.timezone({}))",
                timedelta_repr(offset_seconds)
            ));
        }
    }
    out
}

/// The Python repr of one string, choosing the quote as CPython
/// chooses it and escaping every character that is not printable
/// as the runtime defines printable.
fn string_repr(text: &str) -> String {
    let has_single = text.contains('\'');
    let has_double = text.contains('"');
    let quote = if has_single && !has_double { '"' } else { '\'' };
    let mut out = String::new();
    out.push(quote);
    for ch in text.chars() {
        if ch == quote || ch == '\\' {
            out.push('\\');
            out.push(ch);
        } else if ch == '\n' {
            out.push_str("\\n");
        } else if ch == '\r' {
            out.push_str("\\r");
        } else if ch == '\t' {
            out.push_str("\\t");
        } else if crate::printable::is_printable(ch) {
            out.push(ch);
        } else {
            let code = ch as u32;
            if code < 0x100 {
                out.push_str(&format!("\\x{code:02x}"));
            } else if code < 0x10000 {
                out.push_str(&format!("\\u{code:04x}"));
            } else {
                out.push_str(&format!("\\U{code:08x}"));
            }
        }
    }
    out.push(quote);
    out
}

fn key_repr(key: &YamlKey) -> String {
    match key {
        YamlKey::Null => "None".to_string(),
        YamlKey::Bool(flag) => {
            if *flag {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        YamlKey::Int(integer) => integer.to_string(),
        YamlKey::BigInt(text) => text.clone(),
        YamlKey::Float(number) => python_float_text(*number),
        YamlKey::Text(text) => string_repr(text),
        YamlKey::Date(day) => {
            let (year, month, day_of_month) = civil_from_days(*day);
            format!("datetime.date({year}, {month}, {day_of_month})")
        }
        YamlKey::NaiveDateTime(instant) => datetime_repr(*instant, None),
        YamlKey::AwareDateTime(instant, offset) => datetime_repr(*instant, Some(*offset)),
        YamlKey::Bytes(bytes) => bytes_repr(bytes),
        // A container key spells as the value it stands as
        // spells, the spelling PORT10-3 states, so the one
        // spelling machinery carries both standings.
        YamlKey::List(_) | YamlKey::Mapping(_) | YamlKey::Set(_) | YamlKey::Pairs(_) => {
            python_repr(&key_to_value(key))
        }
    }
}

/// The runtime's repr of a bytes value, the letter b before a
/// quoted spelling of the bytes. The quote stands as a single
/// quote unless the bytes carry a single quote and no double
/// quote, printable ASCII bytes stand as themselves, the
/// backslash and the quote in force are escaped, the newline
/// and carriage return and tab stand as their letter escapes,
/// and every other byte stands as a hexadecimal escape.
fn bytes_repr(bytes: &[u8]) -> String {
    let has_single = bytes.contains(&b'\'');
    let has_double = bytes.contains(&b'"');
    let quote = if has_single && !has_double {
        b'"'
    } else {
        b'\''
    };
    let mut out = String::from("b");
    out.push(quote as char);
    for byte in bytes {
        if *byte == quote || *byte == b'\\' {
            out.push('\\');
            out.push(*byte as char);
        } else if *byte == b'\n' {
            out.push_str("\\n");
        } else if *byte == b'\r' {
            out.push_str("\\r");
        } else if *byte == b'\t' {
            out.push_str("\\t");
        } else if (0x20..0x7f).contains(byte) {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("\\x{byte:02x}"));
        }
    }
    out.push(quote as char);
    out
}

/// The Python repr of a YAML value, as the reconciliation reads
/// it when a container value is asked for as a string.
pub fn python_repr(value: &YamlValue) -> String {
    match value {
        YamlValue::List(items) => {
            let parts: Vec<String> = items.iter().map(python_repr).collect();
            format!("[{}]", parts.join(", "))
        }
        YamlValue::Mapping(pairs) => {
            let parts: Vec<String> = pairs
                .iter()
                .map(|(key, item)| format!("{}: {}", key_repr(key), python_repr(item)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        YamlValue::Bytes(bytes) => bytes_repr(bytes),
        YamlValue::Set(members) => {
            if members.is_empty() {
                "set()".to_string()
            } else {
                // The recorded spelling of a set under
                // PORT9-5, its members' spellings sorted, the
                // runtime's own order standing seeded afresh
                // in each of its runs.
                let mut parts: Vec<String> = members.iter().map(key_repr).collect();
                parts.sort();
                format!("{{{}}}", parts.join(", "))
            }
        }
        YamlValue::Pairs(pairs) => {
            let parts: Vec<String> = pairs
                .iter()
                .map(|(key, item)| format!("({}, {})", key_repr(key), python_repr(item)))
                .collect();
            format!("[{}]", parts.join(", "))
        }
        YamlValue::Text(text) => string_repr(text),
        YamlValue::Date(day) => {
            let (year, month, day_of_month) = civil_from_days(*day);
            format!("datetime.date({year}, {month}, {day_of_month})")
        }
        YamlValue::NaiveDateTime(instant) => datetime_repr(*instant, None),
        YamlValue::AwareDateTime(instant, offset) => datetime_repr(*instant, Some(*offset)),
        _ => python_str(value),
    }
}

// ----------------------------------------------------------------
// Plain scalar resolution
// ----------------------------------------------------------------

fn bigint_from_digits(digits: &[u32], base: u32, negative: bool) -> String {
    // Multiply accumulate on decimal digits.
    let mut decimal: Vec<u8> = vec![0];
    for digit in digits {
        let mut carry = *digit;
        for place in decimal.iter_mut() {
            let current = (*place as u32) * base + carry;
            *place = (current % 10) as u8;
            carry = current / 10;
        }
        while carry > 0 {
            decimal.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while decimal.len() > 1 && *decimal.last().expect("digits") == 0 {
        decimal.pop();
    }
    let text: String = decimal
        .iter()
        .rev()
        .map(|digit| (b'0' + digit) as char)
        .collect();
    if negative && text != "0" {
        format!("-{text}")
    } else {
        text
    }
}

fn integer_value(text: String) -> YamlValue {
    match text.parse::<i64>() {
        Ok(integer) => YamlValue::Int(integer),
        Err(_) => YamlValue::BigInt(text),
    }
}

fn digits_of(text: &str, valid: fn(char) -> bool) -> Option<Vec<u32>> {
    let mut digits = Vec::new();
    for ch in text.chars() {
        if ch == '_' {
            continue;
        }
        if !valid(ch) {
            return None;
        }
        digits.push(ch.to_digit(16)?);
    }
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

/// Resolve an integer shaped body, already split from its sign,
/// or hand back nothing when the body is not an integer form.
fn resolve_integer_body(body: &str, negative: bool) -> Option<YamlValue> {
    if let Some(rest) = body.strip_prefix("0b") {
        let digits = digits_of(rest, |ch| matches!(ch, '0' | '1' | '_'))?;
        return Some(integer_value(bigint_from_digits(&digits, 2, negative)));
    }
    if let Some(rest) = body.strip_prefix("0x") {
        let digits = digits_of(rest, |ch| ch.is_ascii_hexdigit() || ch == '_')?;
        return Some(integer_value(bigint_from_digits(&digits, 16, negative)));
    }
    if body.contains(':') {
        // A base sixty integer, its first group opening on a
        // nonzero digit and each later group under sixty.
        let groups: Vec<&str> = body.split(':').collect();
        let first = groups[0];
        if first.is_empty()
            || !first.chars().next().expect("group").is_ascii_digit()
            || first.starts_with('0')
        {
            return None;
        }
        let mut place_values: Vec<u32> = Vec::new();
        for (index, group) in groups.iter().enumerate() {
            if index > 0 && !sexagesimal_group_stands(group) {
                return None;
            }
            let cleaned: String = group.chars().filter(|ch| *ch != '_').collect();
            if cleaned.is_empty() || !cleaned.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            let value: i64 = cleaned.parse().ok()?;
            place_values.push(value as u32);
        }
        let mut text = "0".to_string();
        for value in &place_values {
            text = add_decimal_text(&multiply_decimal_text(&text, 60), *value);
        }
        if negative && text != "0" {
            text = format!("-{text}");
        }
        return Some(integer_value(text));
    }
    if body.len() > 1
        && body.starts_with('0')
        && body
            .chars()
            .all(|ch| ('0'..='7').contains(&ch) || ch == '_')
    {
        let digits = digits_of(body, |ch| ('0'..='7').contains(&ch) || ch == '_')?;
        return Some(integer_value(bigint_from_digits(&digits, 8, negative)));
    }
    if body == "0"
        || (body
            .chars()
            .next()
            .is_some_and(|ch| ('1'..='9').contains(&ch))
            && body.chars().all(|ch| ch.is_ascii_digit() || ch == '_'))
    {
        let cleaned: String = body.chars().filter(|ch| *ch != '_').collect();
        let text = if negative && cleaned != "0" {
            format!("-{cleaned}")
        } else {
            cleaned
        };
        return Some(integer_value(text));
    }
    None
}

/// Multiply a nonnegative decimal string by a small factor.
fn multiply_decimal_text(text: &str, factor: u32) -> String {
    let mut carry = 0u32;
    let mut out: Vec<u8> = Vec::new();
    for ch in text.chars().rev() {
        let current = ch.to_digit(10).expect("digit") * factor + carry;
        out.push((current % 10) as u8);
        carry = current / 10;
    }
    while carry > 0 {
        out.push((carry % 10) as u8);
        carry /= 10;
    }
    while out.len() > 1 && *out.last().expect("digit") == 0 {
        out.pop();
    }
    out.iter()
        .rev()
        .map(|digit| (b'0' + digit) as char)
        .collect()
}

/// Add a small nonnegative integer to a nonnegative decimal string.
fn add_decimal_text(text: &str, addend: u32) -> String {
    let mut digits: Vec<u8> = text
        .chars()
        .map(|ch| ch.to_digit(10).expect("digit") as u8)
        .collect();
    let mut carry = addend;
    let mut index = digits.len();
    while carry > 0 {
        if index == 0 {
            digits.insert(0, 0);
            index = 1;
        }
        index -= 1;
        let current = digits[index] as u32 + carry;
        digits[index] = (current % 10) as u8;
        carry = current / 10;
    }
    digits.iter().map(|digit| (b'0' + digit) as char).collect()
}

/// Whether a group after the first in a base sixty spelling
/// stands as the resolver pattern stands it, one or two digits
/// with no underscores and a two digit group opening under six.
fn sexagesimal_group_stands(group: &str) -> bool {
    let chars: Vec<char> = group.chars().collect();
    match chars.len() {
        1 => chars[0].is_ascii_digit(),
        2 => ('0'..='5').contains(&chars[0]) && chars[1].is_ascii_digit(),
        _ => false,
    }
}

/// Resolve a float shaped body, split from its sign, or hand
/// back nothing when the body is not a float form.
fn resolve_float_body(body: &str, negative: bool) -> Option<YamlValue> {
    if body.contains(':') {
        // A base sixty float, the fraction standing after the
        // last group and every earlier group a whole number.
        let groups: Vec<&str> = body.split(':').collect();
        if groups.len() < 2 {
            return None;
        }
        for group in &groups[..groups.len() - 1] {
            if group.is_empty()
                || !group.chars().next().is_some_and(|ch| ch.is_ascii_digit())
                || !group.chars().all(|ch| ch.is_ascii_digit() || ch == '_')
            {
                return None;
            }
        }
        let last = groups[groups.len() - 1];
        let (seconds_text, fraction_text) = last.split_once('.')?;
        if !sexagesimal_group_stands(seconds_text) {
            return None;
        }
        for group in &groups[1..groups.len() - 1] {
            if !sexagesimal_group_stands(group) {
                return None;
            }
        }
        let mut total: f64 = 0.0;
        for (index, group) in groups.iter().enumerate() {
            let head = if index == groups.len() - 1 {
                seconds_text
            } else {
                group
            };
            let cleaned: String = head.chars().filter(|ch| *ch != '_').collect();
            if cleaned.is_empty() || !cleaned.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            let value: i64 = cleaned.parse().ok()?;
            total = total * 60.0 + value as f64;
        }
        let fraction_digits: String = fraction_text.chars().filter(|ch| *ch != '_').collect();
        if !fraction_digits.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        let fraction_value: f64 = format!("0.{fraction_digits}").parse().ok()?;
        let total = total + fraction_value;
        return Some(YamlValue::Float(if negative { -total } else { total }));
    }
    // A decimal float, the point standing somewhere and an
    // exponent only with its sign.
    let mantissa = match body.find(['e', 'E']) {
        Some(index) => {
            let exponent_text = &body[index + 1..];
            if !(exponent_text.starts_with('+') || exponent_text.starts_with('-')) {
                return None;
            }
            &body[..index]
        }
        None => body,
    };
    let (whole, fraction) = mantissa.split_once('.')?;
    let whole_ok = whole.is_empty()
        || (whole.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            && whole.chars().all(|ch| ch.is_ascii_digit() || ch == '_'));
    let fraction_ok = fraction.chars().all(|ch| ch.is_ascii_digit() || ch == '_');
    if !whole_ok || !fraction_ok {
        return None;
    }
    if whole.is_empty()
        && !fraction
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    let mut cleaned: String = body.chars().filter(|ch| *ch != '_').collect();
    if cleaned.starts_with('.') {
        cleaned = format!("0{cleaned}");
    }
    if let Ok(value) = cleaned.parse::<f64>() {
        Some(YamlValue::Float(if negative { -value } else { value }))
    } else {
        None
    }
}

/// Read a date or timestamp shaped scalar, or hand back nothing
/// when the shape does not stand. A shape that stands with a
/// value that does not refuses the whole document in the
/// caller, as the Python command dies on it.
enum TimestampRead {
    NotTimestamp,
    Refused,
    Date(i64),
    Naive(i128),
    Aware(i128, i64),
}

fn take_digits(chars: &[char], pos: &mut usize, min: usize, max: usize) -> Option<i64> {
    let mut value: i64 = 0;
    let mut count = 0;
    while count < max {
        match chars.get(*pos) {
            Some(ch) if ch.is_ascii_digit() => {
                value = value * 10 + (*ch as u32 - '0' as u32) as i64;
                *pos += 1;
                count += 1;
            }
            _ => break,
        }
    }
    if count < min {
        None
    } else {
        Some(value)
    }
}

fn read_timestamp(text: &str) -> TimestampRead {
    let chars: Vec<char> = text.chars().collect();
    // The date alone, two digit month and day.
    if chars.len() == 10 && chars[4] == '-' && chars[7] == '-' {
        let mut cursor = 0;
        let year = match take_digits(&chars, &mut cursor, 4, 4) {
            Some(value) => value,
            None => return TimestampRead::NotTimestamp,
        };
        if chars.get(cursor) != Some(&'-') {
            return TimestampRead::NotTimestamp;
        }
        cursor += 1;
        let month = match take_digits(&chars, &mut cursor, 2, 2) {
            Some(value) => value,
            None => return TimestampRead::NotTimestamp,
        };
        if chars.get(cursor) != Some(&'-') {
            return TimestampRead::NotTimestamp;
        }
        cursor += 1;
        let day = match take_digits(&chars, &mut cursor, 2, 2) {
            Some(value) => value,
            None => return TimestampRead::NotTimestamp,
        };
        if cursor != chars.len() {
            return TimestampRead::NotTimestamp;
        }
        if !(1..=9999).contains(&year) || !valid_calendar_date(year, month, day) {
            return TimestampRead::Refused;
        }
        return TimestampRead::Date(days_from_civil(year, month, day));
    }
    // A date with a time.
    let mut cursor = 0;
    let year = match take_digits(&chars, &mut cursor, 4, 4) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    if chars.get(cursor) != Some(&'-') {
        return TimestampRead::NotTimestamp;
    }
    cursor += 1;
    let month = match take_digits(&chars, &mut cursor, 1, 2) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    if chars.get(cursor) != Some(&'-') {
        return TimestampRead::NotTimestamp;
    }
    cursor += 1;
    let day = match take_digits(&chars, &mut cursor, 1, 2) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    match chars.get(cursor) {
        Some('T') | Some('t') => cursor += 1,
        Some(' ') | Some('\t') => {
            while matches!(chars.get(cursor), Some(' ') | Some('\t')) {
                cursor += 1;
            }
        }
        _ => return TimestampRead::NotTimestamp,
    }
    let hour = match take_digits(&chars, &mut cursor, 1, 2) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    if chars.get(cursor) != Some(&':') {
        return TimestampRead::NotTimestamp;
    }
    cursor += 1;
    let minute = match take_digits(&chars, &mut cursor, 2, 2) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    if chars.get(cursor) != Some(&':') {
        return TimestampRead::NotTimestamp;
    }
    cursor += 1;
    let second = match take_digits(&chars, &mut cursor, 2, 2) {
        Some(value) => value,
        None => return TimestampRead::NotTimestamp,
    };
    let mut micros: i64 = 0;
    if chars.get(cursor) == Some(&'.') {
        cursor += 1;
        let start = cursor;
        while chars.get(cursor).is_some_and(|ch| ch.is_ascii_digit()) {
            cursor += 1;
        }
        let digits = &chars[start..cursor];
        let mut places = 0;
        for digit in digits.iter().take(6) {
            micros = micros * 10 + (*digit as u32 - '0' as u32) as i64;
            places += 1;
        }
        while places < 6 {
            micros *= 10;
            places += 1;
        }
    }
    while matches!(chars.get(cursor), Some(' ') | Some('\t')) {
        cursor += 1;
    }
    let mut offset_seconds: Option<i64> = None;
    if cursor < chars.len() {
        match chars[cursor] {
            'Z' => {
                cursor += 1;
                offset_seconds = Some(0);
            }
            '+' | '-' => {
                let sign = if chars[cursor] == '-' { -1 } else { 1 };
                cursor += 1;
                let tz_hour = match take_digits(&chars, &mut cursor, 1, 2) {
                    Some(value) => value,
                    None => return TimestampRead::NotTimestamp,
                };
                let mut tz_minute = 0;
                if chars.get(cursor) == Some(&':') {
                    cursor += 1;
                    tz_minute = match take_digits(&chars, &mut cursor, 2, 2) {
                        Some(value) => value,
                        None => return TimestampRead::NotTimestamp,
                    };
                }
                offset_seconds = Some(sign * (tz_hour * 3600 + tz_minute * 60));
            }
            _ => return TimestampRead::NotTimestamp,
        }
    }
    if cursor != chars.len() {
        return TimestampRead::NotTimestamp;
    }
    if !(1..=9999).contains(&year)
        || !valid_calendar_date(year, month, day)
        || hour > 23
        || minute > 59
        || second > 59
        || offset_seconds.is_some_and(|offset| offset.abs() >= 86_400)
    {
        return TimestampRead::Refused;
    }
    let day_number = days_from_civil(year, month, day);
    let local = day_number as i128 * 86_400_000_000
        + (hour * 3600 + minute * 60 + second) as i128 * 1_000_000
        + micros as i128;
    match offset_seconds {
        None => TimestampRead::Naive(local),
        Some(offset) => TimestampRead::Aware(local - offset as i128 * 1_000_000, offset),
    }
}

/// Resolve a plain scalar into its value, or refuse the
/// document when a timestamp shape carries a value that does
/// not stand.
pub fn resolve_plain_scalar(text: &str) -> Option<YamlValue> {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Some(YamlValue::Null),
        "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" | "on" | "On" | "ON" => {
            return Some(YamlValue::Bool(true))
        }
        "no" | "No" | "NO" | "false" | "False" | "FALSE" | "off" | "Off" | "OFF" => {
            return Some(YamlValue::Bool(false))
        }
        ".inf" | ".Inf" | ".INF" => return Some(YamlValue::Float(f64::INFINITY)),
        "-.inf" | "-.Inf" | "-.INF" => return Some(YamlValue::Float(f64::NEG_INFINITY)),
        "+.inf" | "+.Inf" | "+.INF" => return Some(YamlValue::Float(f64::INFINITY)),
        ".nan" | ".NaN" | ".NAN" => return Some(YamlValue::Float(f64::NAN)),
        _ => {}
    }
    let (negative, body) = if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = text.strip_prefix('+') {
        (false, rest)
    } else {
        (false, text)
    };
    if !body.is_empty() {
        if let Some(value) = resolve_integer_body(body, negative) {
            return Some(value);
        }
        // A sign standing directly in front of the point never
        // resolves as a float in the runtime resolution, so a
        // body opening with the point after a sign stays a
        // string. An unsigned body opening with the point still
        // resolves, as the recorded floats show.
        let signed = body.len() != text.len();
        if !(signed && body.starts_with('.')) {
            if let Some(value) = resolve_float_body(body, negative) {
                return Some(value);
            }
        }
    }
    match read_timestamp(text) {
        TimestampRead::NotTimestamp => Some(YamlValue::Text(text.to_string())),
        TimestampRead::Refused => None,
        TimestampRead::Date(day) => Some(YamlValue::Date(day)),
        TimestampRead::Naive(instant) => Some(YamlValue::NaiveDateTime(instant)),
        TimestampRead::Aware(instant, offset) => Some(YamlValue::AwareDateTime(instant, offset)),
    }
}

// ----------------------------------------------------------------
// The document reader
// ----------------------------------------------------------------

/// The kind of a break standing behind a line, the line feed
/// kind and the two separator kinds of PORT15-1. A break of a
/// separator kind renders as the character itself wherever a
/// reading renders the break standing behind a line, where a
/// break of the line feed kind renders as the one line feed.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BreakKind {
    LineFeed,
    LineSeparator,
    ParagraphSeparator,
}

impl BreakKind {
    /// The character the break renders as, the separator
    /// itself for a separator kind and the line feed for the
    /// line feed kind.
    fn as_char(self) -> char {
        match self {
            BreakKind::LineFeed => '\n',
            BreakKind::LineSeparator => '\u{2028}',
            BreakKind::ParagraphSeparator => '\u{2029}',
        }
    }

    /// Whether the break is one of the two separators.
    fn is_separator(self) -> bool {
        !matches!(self, BreakKind::LineFeed)
    }
}

/// Fold a run of breaks under the rule PORT15-2 states. A run
/// of one break folds to one space where the break is of the
/// line feed kind and to the separator itself where it is a
/// separator. A run of two or more drops its first break where
/// the first is of the line feed kind and keeps every break
/// where the first is a separator, and every break after the
/// first stands as a line feed where it is of the line feed
/// kind and as itself where it is a separator.
fn fold_break_run(run: &[BreakKind]) -> String {
    if run.len() == 1 {
        return match run[0] {
            BreakKind::LineFeed => " ".to_string(),
            kind => kind.as_char().to_string(),
        };
    }
    let mut out = String::new();
    for (index, kind) in run.iter().enumerate() {
        if index == 0 && !kind.is_separator() {
            continue;
        }
        out.push(kind.as_char());
    }
    out
}

#[derive(Debug, Clone)]
struct Line {
    indent: usize,
    text: String,
    /// The whole raw line, its leading spaces standing in it,
    /// as the block scalar reading consumes it.
    raw: String,
    /// Whether a line break stands behind this line in the
    /// source, the last line of the text carrying none.
    has_break: bool,
    /// The kind of the break standing behind this line, read
    /// only where a break stands, the kind of a line carrying
    /// no break never read.
    kind: BreakKind,
}

struct Cursor {
    lines: Vec<Line>,
    pos: usize,
}

impl Cursor {
    /// The index of the next line at or after `from` that carries
    /// content, a blank line and a comment only line skipped.
    fn next_content(&self, from: usize) -> Option<usize> {
        let mut index = from;
        while index < self.lines.len() {
            let text = self.lines[index].text.trim();
            if !text.is_empty() && !text.starts_with('#') {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn peek_content(&self) -> Option<usize> {
        self.next_content(self.pos)
    }
}

fn is_dash_text(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn is_dash_line(line: &Line) -> bool {
    is_dash_text(&line.text)
}

fn insert_pair(pairs: &mut Vec<(YamlKey, YamlValue)>, key: YamlKey, value: YamlValue) {
    match pairs.iter_mut().find(|(standing, _)| *standing == key) {
        Some((_, standing)) => *standing = value,
        None => pairs.push((key, value)),
    }
}

// ----------------------------------------------------------------
// Anchors, aliases, merge keys, and tags, PORT4
// ----------------------------------------------------------------

/// A tag standing on a node, as the expansion of its spelling
/// under the standing bindings names it. The standard scalar
/// and collection tags stand apart, the merge tag is a key's
/// tag only, and every other expansion refuses.
#[derive(Clone, PartialEq)]
enum Tag {
    None,
    NonSpecific,
    Str,
    Int,
    Float,
    Bool,
    Null,
    Timestamp,
    Seq,
    Map,
    Merge,
    Binary,
    Set,
    Omap,
    Pairs,
    Refuse,
}

impl Tag {
    fn is_constructor(&self) -> bool {
        matches!(
            self,
            Tag::Str | Tag::Int | Tag::Float | Tag::Bool | Tag::Null | Tag::Timestamp | Tag::Binary
        )
    }
}

/// The properties standing in front of a node, at most one
/// anchor and at most one tag in either order.
#[derive(Clone)]
struct Props {
    anchor: Option<String>,
    tag: Tag,
}

impl Props {
    fn new() -> Props {
        Props {
            anchor: None,
            tag: Tag::None,
        }
    }
}

/// A value the reader has built, carrying the tally of the
/// expansion costs inside it, the tally of each item when it
/// is a list, and the source text when it is a scalar node,
/// so a tag standing outside the node's own lines can still
/// convert it.
#[derive(Clone)]
struct Built {
    value: YamlValue,
    tally: usize,
    items: Option<Vec<usize>>,
    text: Option<String>,
    plain_token: Option<String>,
    tag_done: bool,
}

impl Built {
    fn scalar(value: YamlValue) -> Built {
        Built {
            value,
            tally: 0,
            items: None,
            text: None,
            plain_token: None,
            tag_done: false,
        }
    }

    /// The empty scalar node, a null under plain resolution
    /// whose text a tag can still convert.
    fn empty_scalar() -> Built {
        Built {
            value: YamlValue::Null,
            tally: 0,
            items: None,
            text: Some(String::new()),
            plain_token: None,
            tag_done: false,
        }
    }

    fn from_text(text: String) -> Built {
        Built {
            value: YamlValue::Text(text.clone()),
            tally: 0,
            items: None,
            text: Some(text),
            plain_token: None,
            tag_done: false,
        }
    }

    fn converted(value: YamlValue) -> Built {
        Built {
            value,
            tally: 0,
            items: None,
            text: None,
            plain_token: None,
            tag_done: true,
        }
    }

    fn list(items: Vec<Built>) -> Built {
        let tallies: Vec<usize> = items.iter().map(|item| item.tally).collect();
        let tally = tallies.iter().sum();
        Built {
            value: YamlValue::List(items.into_iter().map(|item| item.value).collect()),
            tally,
            items: Some(tallies),
            text: None,
            plain_token: None,
            tag_done: false,
        }
    }
}

/// A value bound under an anchor, with the tally the value
/// cost to build, so a copy of it counts what the copy
/// resolves inside it. The record also carries whether
/// the value stood as a merge key where it was bound, by
/// the plain spelling or under the merge tag, the standing
/// PORT13-3 tells apart at an alias standing as a key.
#[derive(Clone)]
struct AnchorVal {
    value: YamlValue,
    tally: usize,
    items: Option<Vec<usize>>,
    merge_key: bool,
}

/// The expansion bound of PORT4, standing across one
/// document.
const EXPANSION_BOUND: usize = 4000;

/// The state one document's reading carries, its tag
/// bindings, its anchor table, the anchors whose nodes are
/// still being built, and its expansion count.
struct ReadState {
    bindings: HashMap<String, String>,
    bound: HashMap<String, AnchorVal>,
    building: HashSet<String>,
    expansions: usize,
}

impl ReadState {
    /// Mark an anchor's node as under construction, refusing
    /// a name that stands bound or building already.
    fn mark(&mut self, name: &str) -> Option<()> {
        if self.bound.contains_key(name) || self.building.contains(name) {
            return None;
        }
        self.building.insert(name.to_string());
        Some(())
    }

    fn bind(&mut self, name: &str, built: &Built) {
        self.building.remove(name);
        self.bound.insert(
            name.to_string(),
            AnchorVal {
                value: built.value.clone(),
                tally: built.tally,
                items: built.items.clone(),
                merge_key: built.plain_token.as_deref() == Some("<<"),
            },
        );
    }

    fn charge(&mut self, cost: usize) -> Option<()> {
        self.expansions += cost;
        if self.expansions > EXPANSION_BOUND {
            None
        } else {
            Some(())
        }
    }

    /// Read an alias as a copy of its anchored value,
    /// charging the copy and what the copy resolves inside
    /// it. An alias naming a node still under construction
    /// is the recursive alias and refuses.
    fn resolve_alias(&mut self, name: &str) -> Option<Built> {
        if self.building.contains(name) {
            return None;
        }
        let entry = self.bound.get(name)?.clone();
        let cost = 1 + entry.tally;
        self.charge(cost)?;
        Some(Built {
            value: entry.value,
            tally: cost,
            items: entry.items,
            text: None,
            plain_token: None,
            tag_done: false,
        })
    }
}

/// Expand one merge source into the pairs a carrying mapping
/// gathers ahead of its explicit pairs, charging the
/// expansion and what the copying resolved inside the
/// source as it was built.
fn expand_merge_into(
    state: &mut ReadState,
    source: Built,
    out: &mut Vec<(YamlKey, YamlValue)>,
    tally: &mut usize,
) -> Option<()> {
    match source.value {
        YamlValue::Mapping(pairs) => {
            let cost = 1 + source.tally;
            state.charge(cost)?;
            *tally += cost;
            out.extend(pairs);
            Some(())
        }
        YamlValue::List(items) => {
            let tallies = source.items?;
            if tallies.len() != items.len() {
                return None;
            }
            let mut sources: Vec<Vec<(YamlKey, YamlValue)>> = Vec::new();
            for (item, item_tally) in items.into_iter().zip(tallies.iter()) {
                match item {
                    YamlValue::Mapping(pairs) => {
                        let cost = 1 + item_tally;
                        state.charge(cost)?;
                        *tally += cost;
                        sources.push(pairs);
                    }
                    _ => return None,
                }
            }
            // The pairs of a list of sources stand in the
            // reverse of the list's order, the earlier listed
            // source winning the repeated keys by standing in
            // front of them.
            for pairs in sources.into_iter().rev() {
                out.extend(pairs);
            }
            Some(())
        }
        _ => None,
    }
}

/// Gather the pairs of a mapping from its merge sources and
/// its explicit pairs, the merged pairs standing in front of
/// every explicit pair wherever the merge key was written.
fn flatten_mapping(
    state: &mut ReadState,
    merge_sources: Vec<Built>,
    explicit: Vec<(YamlKey, Built)>,
) -> Option<Built> {
    let mut merged: Vec<(YamlKey, YamlValue)> = Vec::new();
    let mut tally = 0usize;
    for source in merge_sources {
        expand_merge_into(state, source, &mut merged, &mut tally)?;
    }
    let mut pairs: Vec<(YamlKey, YamlValue)> = Vec::new();
    for (key, value) in merged {
        insert_pair(&mut pairs, key, value);
    }
    for (key, built) in explicit {
        tally += built.tally;
        insert_pair(&mut pairs, key, built.value);
    }
    Some(Built {
        value: YamlValue::Mapping(pairs),
        tally,
        items: None,
        text: None,
        plain_token: None,
        tag_done: false,
    })
}

fn is_anchor_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

fn is_anchor_char_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

fn classify_tag(full: &str) -> Tag {
    match full {
        "tag:yaml.org,2002:str" => Tag::Str,
        "tag:yaml.org,2002:int" => Tag::Int,
        "tag:yaml.org,2002:float" => Tag::Float,
        "tag:yaml.org,2002:bool" => Tag::Bool,
        "tag:yaml.org,2002:null" => Tag::Null,
        "tag:yaml.org,2002:timestamp" => Tag::Timestamp,
        "tag:yaml.org,2002:seq" => Tag::Seq,
        "tag:yaml.org,2002:map" => Tag::Map,
        "tag:yaml.org,2002:merge" => Tag::Merge,
        "tag:yaml.org,2002:binary" => Tag::Binary,
        "tag:yaml.org,2002:set" => Tag::Set,
        "tag:yaml.org,2002:omap" => Tag::Omap,
        "tag:yaml.org,2002:pairs" => Tag::Pairs,
        _ => Tag::Refuse,
    }
}

/// Resolve a tag token spelling to the tag it names under
/// the standing bindings, refusing a handle no directive
/// bound.
fn resolve_tag_spelling(token: &str, bindings: &HashMap<String, String>) -> Option<Tag> {
    if token == "!" {
        return Some(Tag::NonSpecific);
    }
    let (handle, suffix) = if let Some(rest) = token.strip_prefix("!!") {
        ("!!", rest)
    } else if let Some(index) = token[1..].find('!') {
        (&token[..index + 2], &token[index + 2..])
    } else {
        ("!", &token[1..])
    };
    let prefix = bindings.get(handle)?;
    Some(classify_tag(&format!("{prefix}{suffix}")))
}

/// Read the tag token standing at the head of `text` from
/// byte `pos`, handing back the tag and the byte just past
/// the token. A verbatim spelling names its tag inside angle
/// brackets, and an empty verbatim spelling refuses.
fn read_tag_token(
    text: &str,
    pos: usize,
    bindings: &HashMap<String, String>,
) -> Option<(Tag, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(pos + 1) == Some(&b'<') {
        let mut end = pos + 2;
        while end < bytes.len() && bytes[end] != b'>' {
            end += 1;
        }
        if end >= bytes.len() {
            return None;
        }
        let full = &text[pos + 2..end];
        if full.is_empty() {
            return None;
        }
        let after = end + 1;
        match bytes.get(after) {
            None | Some(b' ') => Some((classify_tag(full), after)),
            _ => None,
        }
    } else {
        let mut end = pos;
        while end < bytes.len() && bytes[end] != b' ' && bytes[end] != b'\t' {
            end += 1;
        }
        let tag = resolve_tag_spelling(&text[pos..end], bindings)?;
        Some((tag, end))
    }
}

enum PropSplit {
    None,
    Props(Props, usize),
    Refuse,
}

/// Split the properties standing at the head of a node text
/// in block position, at most one anchor and at most one tag
/// in either order. The byte handed back stands at the
/// node's own text.
fn split_properties(text: &str, bindings: &HashMap<String, String>) -> PropSplit {
    let bytes = text.as_bytes();
    if !matches!(bytes.first(), Some(b'&') | Some(b'!')) {
        return PropSplit::None;
    }
    let mut props = Props::new();
    let mut pos = 0usize;
    loop {
        match bytes[pos] {
            b'&' => {
                if props.anchor.is_some() {
                    return PropSplit::Refuse;
                }
                let start = pos + 1;
                let mut end = start;
                while end < bytes.len() && is_anchor_char(bytes[end]) {
                    end += 1;
                }
                if end == start {
                    return PropSplit::Refuse;
                }
                match bytes.get(end) {
                    None | Some(b' ') => {}
                    _ => return PropSplit::Refuse,
                }
                props.anchor = Some(text[start..end].to_string());
                pos = end;
            }
            b'!' => {
                if props.tag != Tag::None {
                    return PropSplit::Refuse;
                }
                match read_tag_token(text, pos, bindings) {
                    Some((tag, end)) => {
                        props.tag = tag;
                        pos = end;
                    }
                    None => return PropSplit::Refuse,
                }
            }
            _ => return PropSplit::Refuse,
        }
        let mut next = pos;
        while bytes.get(next) == Some(&b' ') {
            next += 1;
        }
        if matches!(bytes.get(next), Some(b'&') | Some(b'!')) {
            pos = next;
            continue;
        }
        return PropSplit::Props(props, next);
    }
}

/// The standing a tag has in key position, where the merge
/// tag merges and the non-specific tag reads as the plain
/// key reads, every other tag refusing.
#[derive(Clone, PartialEq)]
enum KeyTag {
    None,
    NonSpecific,
    Merge,
    Convert(Tag),
    Refuse,
}

/// The class a tag stands in when it stands in front of a
/// key, the scalar tags standing as conversions the key
/// scalar takes and the collection tags and the refused
/// tags standing refused.
fn key_tag_of(tag: &Tag) -> KeyTag {
    match tag {
        Tag::None => KeyTag::None,
        Tag::NonSpecific => KeyTag::NonSpecific,
        Tag::Merge => KeyTag::Merge,
        Tag::Str | Tag::Int | Tag::Float | Tag::Bool | Tag::Null | Tag::Timestamp | Tag::Binary => {
            KeyTag::Convert(tag.clone())
        }
        _ => KeyTag::Refuse,
    }
}

struct KeyProps {
    anchor: Option<String>,
    tag: KeyTag,
}

enum KeySplit {
    None,
    Props(KeyProps, usize),
    Refuse,
}

/// Split the properties standing at the head of a key text in
/// block position, the tag classes of key position riding on
/// the split.
fn split_key_properties(text: &str, bindings: &HashMap<String, String>) -> KeySplit {
    match split_properties(text, bindings) {
        PropSplit::None => KeySplit::None,
        PropSplit::Refuse => KeySplit::Refuse,
        PropSplit::Props(props, offset) => {
            let tag = key_tag_of(&props.tag);
            KeySplit::Props(
                KeyProps {
                    anchor: props.anchor,
                    tag,
                },
                offset,
            )
        }
    }
}

/// Whether a line's text carries a mapping key, a property
/// standing in front of the key counted as standing in front
/// of the key.
fn mapping_shape(text: &str, bindings: &HashMap<String, String>) -> bool {
    match split_key_properties(text, bindings) {
        KeySplit::Props(_, offset) => {
            find_key_split(&text[offset..]).is_some()
                || flow_key_split(&text[offset..]).is_some()
                || explicit_opening(&text[offset..])
                || alias_key_shape(&text[offset..])
        }
        KeySplit::None => {
            find_key_split(text).is_some()
                || flow_key_split(text).is_some()
                || explicit_opening(text)
                || alias_key_shape(text)
        }
        KeySplit::Refuse => false,
    }
}

/// Whether a line's text carries a mapping pair of its own,
/// a property standing in front of the pair counted as
/// standing in front of the pair. The test differs from the
/// mapping shape test at one sub shape alone. A bare
/// explicit opening, the question mark standing alone on
/// the line or in front of a space with no pair standing
/// behind it on the line, carries no pair, and an opening
/// whose key standing behind the question mark carries a
/// pair of its own carries that pair, read by this same
/// test on the text standing behind the opening.
fn mapping_pair_shape(text: &str, bindings: &HashMap<String, String>) -> bool {
    let rest = match split_key_properties(text, bindings) {
        KeySplit::Props(_, offset) => &text[offset..],
        KeySplit::None => text,
        KeySplit::Refuse => return false,
    };
    if find_key_split(rest).is_some() || flow_key_split(rest).is_some() || alias_key_shape(rest) {
        return true;
    }
    match rest.strip_prefix("? ") {
        Some(behind) => mapping_pair_shape(behind, bindings),
        None => false,
    }
}

/// Where a flow collection standing as a compact key ends on
/// its line, as the byte index just past the collection and
/// the byte index just past the pair's colon, when the text
/// opens with a flow collection and the pair's colon stands
/// behind it with a space or the line's end behind the colon.
/// The walk counts brackets with quoted spans taken into
/// account, a quote opening only where a scalar can stand in
/// flow, at the text's start or behind a bracket or a comma
/// or a colon or a space, so a quote standing inside a plain
/// scalar opens nothing. The walk only routes the line; the
/// flow reading itself still decides what the collection
/// reads as.
fn flow_key_split(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    if !matches!(bytes.first(), Some(b'[') | Some(b'{')) {
        return None;
    }
    let mut depth = 0i64;
    let mut quote: Option<u8> = None;
    let mut prev = 0u8;
    let mut index = 0usize;
    let mut end = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(b'"') => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == b'"' {
                    quote = None;
                }
            }
            Some(b'\'') => {
                if byte == b'\'' {
                    if bytes.get(index + 1) == Some(&b'\'') {
                        index += 1;
                    } else {
                        quote = None;
                    }
                }
            }
            _ => match byte {
                b'"' | b'\''
                    if index == 0 || matches!(prev, b'[' | b'{' | b',' | b':' | b' ' | b'\t') =>
                {
                    quote = Some(byte);
                }
                b'[' | b'{' => depth += 1,
                b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index + 1);
                        break;
                    }
                    if depth < 0 {
                        return None;
                    }
                }
                b'#' if index > 0 && matches!(prev, b' ' | b'\t') => return None,
                _ => {}
            },
        }
        if !matches!(byte, b' ' | b'\t') {
            prev = byte;
        }
        index += 1;
    }
    let end = end?;
    let mut colon = end;
    while colon < bytes.len() && matches!(bytes[colon], b' ' | b'\t') {
        colon += 1;
    }
    if bytes.get(colon) != Some(&b':') {
        return None;
    }
    let after = colon + 1;
    if after < bytes.len() && !matches!(bytes[after], b' ' | b'\t') {
        return None;
    }
    Some((end, after))
}

/// Whether a line's text opens a pair whose key is an alias,
/// the alias token standing in front of the pair's colon.
fn alias_key_shape(text: &str) -> bool {
    let trimmed = text.trim_start();
    if !trimmed.starts_with('*') {
        return false;
    }
    let name_len = trimmed[1..]
        .chars()
        .take_while(|ch| is_anchor_char_char(*ch))
        .map(char::len_utf8)
        .sum::<usize>();
    if name_len == 0 {
        return false;
    }
    let after = trimmed[1 + name_len..].trim_start();
    after.starts_with(':') && (after.len() == 1 || matches!(after.as_bytes()[1], b' ' | b'\t'))
}

/// Whether a line's text opens an explicit pair, the question
/// mark standing alone on the line or in front of a space.
fn explicit_opening(text: &str) -> bool {
    text == "?" || text.starts_with("? ")
}

/// A decimal integer in text form, a sign and the digits
/// without their leading zeros, as Python's int carries one.
struct DecInt {
    negative: bool,
    digits: String,
}

fn dec_normalize(digits: &str) -> String {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Read a decimal integer the way Python's int reads its
/// text, the whitespace at its ends stripped and a sign
/// standing in front.
fn dec_parse(text: &str) -> Option<DecInt> {
    let trimmed = text.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    if body.is_empty() || !body.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(DecInt {
        negative,
        digits: dec_normalize(body),
    })
}

fn dec_magnitude_add(a: &str, b: &str) -> String {
    let a: Vec<u32> = a.bytes().map(|byte| (byte - b'0') as u32).collect();
    let b: Vec<u32> = b.bytes().map(|byte| (byte - b'0') as u32).collect();
    let mut out: Vec<u32> = Vec::new();
    let mut carry = 0u32;
    let mut index = 0usize;
    while index < a.len() || index < b.len() || carry > 0 {
        let mut sum = carry;
        if index < a.len() {
            sum += a[a.len() - 1 - index];
        }
        if index < b.len() {
            sum += b[b.len() - 1 - index];
        }
        out.push(sum % 10);
        carry = sum / 10;
        index += 1;
    }
    let text: String = out
        .iter()
        .rev()
        .map(|digit| (b'0' + *digit as u8) as char)
        .collect();
    dec_normalize(&text)
}

fn dec_magnitude_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a = dec_normalize(a);
    let b = dec_normalize(b);
    a.len().cmp(&b.len()).then_with(|| a.cmp(&b))
}

/// Subtract the smaller magnitude from the larger, the
/// caller holding the order.
fn dec_magnitude_sub(a: &str, b: &str) -> String {
    let a: Vec<i32> = a.bytes().map(|byte| (byte - b'0') as i32).collect();
    let b: Vec<i32> = b.bytes().map(|byte| (byte - b'0') as i32).collect();
    let mut out: Vec<i32> = Vec::new();
    let mut borrow = 0i32;
    let mut index = 0usize;
    while index < a.len() {
        let mut digit = a[a.len() - 1 - index] - borrow;
        if index < b.len() {
            digit -= b[b.len() - 1 - index];
        }
        if digit < 0 {
            digit += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(digit);
        index += 1;
    }
    let text: String = out
        .iter()
        .rev()
        .map(|digit| (b'0' + *digit as u8) as char)
        .collect();
    dec_normalize(&text)
}

fn dec_add(a: DecInt, b: DecInt) -> DecInt {
    if a.digits == "0" {
        return b;
    }
    if b.digits == "0" {
        return a;
    }
    if a.negative == b.negative {
        DecInt {
            negative: a.negative,
            digits: dec_magnitude_add(&a.digits, &b.digits),
        }
    } else {
        match dec_magnitude_cmp(&a.digits, &b.digits) {
            std::cmp::Ordering::Greater => DecInt {
                negative: a.negative,
                digits: dec_magnitude_sub(&a.digits, &b.digits),
            },
            std::cmp::Ordering::Less => DecInt {
                negative: b.negative,
                digits: dec_magnitude_sub(&b.digits, &a.digits),
            },
            std::cmp::Ordering::Equal => DecInt {
                negative: false,
                digits: "0".to_string(),
            },
        }
    }
}

fn dec_mul_small(a: DecInt, factor: u32) -> DecInt {
    if a.digits == "0" || factor == 0 {
        return DecInt {
            negative: false,
            digits: "0".to_string(),
        };
    }
    let digits: Vec<u32> = a.digits.bytes().map(|byte| (byte - b'0') as u32).collect();
    let mut out: Vec<u32> = Vec::new();
    let mut carry = 0u32;
    for digit in digits.iter().rev() {
        let current = digit * factor + carry;
        out.push(current % 10);
        carry = current / 10;
    }
    while carry > 0 {
        out.push(carry % 10);
        carry /= 10;
    }
    DecInt {
        negative: a.negative,
        digits: out
            .iter()
            .rev()
            .map(|digit| (b'0' + *digit as u8) as char)
            .collect(),
    }
}

fn dec_value(value: DecInt) -> YamlValue {
    if value.negative && value.digits != "0" {
        integer_value(format!("-{}", value.digits))
    } else {
        integer_value(value.digits)
    }
}

/// Read an integer written in a base the way Python's int
/// reads one in that base, the whitespace at its ends
/// stripped, a sign standing in front, and the prefix the
/// base carries let stand or fall as the base reads it.
fn dec_parse_base(text: &str, base: u32) -> Option<DecInt> {
    let trimmed = text.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let body = match base {
        2 => body
            .strip_prefix("0b")
            .or_else(|| body.strip_prefix("0B"))
            .unwrap_or(body),
        8 => body
            .strip_prefix("0o")
            .or_else(|| body.strip_prefix("0O"))
            .unwrap_or(body),
        _ => body
            .strip_prefix("0x")
            .or_else(|| body.strip_prefix("0X"))
            .unwrap_or(body),
    };
    let valid: fn(char) -> bool = match base {
        2 => |ch: char| ch == '0' || ch == '1',
        8 => |ch: char| ('0'..='7').contains(&ch),
        _ => |ch: char| ch.is_ascii_hexdigit(),
    };
    let digits = digits_of(body, valid)?;
    Some(DecInt {
        negative,
        digits: bigint_from_digits(&digits, base, false),
    })
}

/// Convert a scalar text under the int tag, mirroring the
/// constructor of the runtime in its own order of readings,
/// the binary and hex spellings in front of the leading zero
/// octal reading and the sexagesimal reading behind it. The
/// tagged grammar stands beside the plain grammar, and its
/// sexagesimal parts stand at any size.
fn convert_int(text: &str) -> Option<YamlValue> {
    let value: String = text.chars().filter(|ch| *ch != '_').collect();
    let first = value.chars().next()?;
    let (negative, rest) = match first {
        '-' => (true, &value[1..]),
        '+' => (false, &value[1..]),
        _ => (false, value.as_str()),
    };
    let parsed = if rest == "0" {
        DecInt {
            negative: false,
            digits: "0".to_string(),
        }
    } else if rest.starts_with("0b") {
        dec_parse_base(rest, 2)?
    } else if rest.starts_with("0x") {
        dec_parse_base(rest, 16)?
    } else if rest.starts_with('0') {
        dec_parse_base(rest, 8)?
    } else if rest.contains(':') {
        let mut total = DecInt {
            negative: false,
            digits: "0".to_string(),
        };
        for part in rest.split(':') {
            let part = dec_parse(part)?;
            total = dec_add(dec_mul_small(total, 60), part);
        }
        total
    } else {
        dec_parse(rest)?
    };
    Some(dec_value(DecInt {
        negative: negative != parsed.negative && parsed.digits != "0",
        digits: parsed.digits,
    }))
}

/// The shape of a decimal float spelling, digits with one
/// point at most and an exponent with digits of its own.
fn python_float_shape(body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut index = 0usize;
    let mut mantissa = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
        mantissa += 1;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            mantissa += 1;
        }
    }
    if mantissa == 0 {
        return false;
    }
    if index < bytes.len() && (bytes[index] == b'e' || bytes[index] == b'E') {
        index += 1;
        if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-') {
            index += 1;
        }
        let mut exponent = 0usize;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            exponent += 1;
        }
        if exponent == 0 {
            return false;
        }
    }
    index == bytes.len()
}

/// Read a float the way Python's float reads its text, the
/// whitespace at its ends stripped and the infinity and not
/// a number spellings standing as words.
fn python_float(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let value = match body {
        "inf" | "infinity" => f64::INFINITY,
        "nan" => f64::NAN,
        _ => {
            if !python_float_shape(body) {
                return None;
            }
            body.parse::<f64>().ok()?
        }
    };
    Some(if negative { -value } else { value })
}

/// Convert a scalar text under the float tag, mirroring the
/// constructor of the runtime, its sexagesimal form summing
/// in the constructor's own reversed order so the rounding
/// stands where the runtime's rounding stands.
fn convert_float(text: &str) -> Option<YamlValue> {
    let value: String = text
        .chars()
        .filter(|ch| *ch != '_')
        .collect::<String>()
        .to_lowercase();
    let first = value.chars().next()?;
    let (negative, rest) = match first {
        '-' => (true, &value[1..]),
        '+' => (false, &value[1..]),
        _ => (false, value.as_str()),
    };
    let parsed = if rest == ".inf" {
        f64::INFINITY
    } else if rest == ".nan" {
        f64::NAN
    } else if rest.contains(':') {
        let mut parts: Vec<f64> = Vec::new();
        for part in rest.split(':') {
            parts.push(python_float(part)?);
        }
        let mut total = 0.0f64;
        let mut base = 1.0f64;
        for part in parts.iter().rev() {
            total += part * base;
            base *= 60.0;
        }
        total
    } else {
        python_float(rest)?
    };
    Some(YamlValue::Float(if negative { -parsed } else { parsed }))
}

/// Convert a scalar text under the timestamp tag. The tag
/// grammar takes the month and day fields at one or two
/// digits in the date alone form, where the plain grammar
/// asks two, and stands beside the plain grammar otherwise.
fn convert_timestamp(text: &str) -> Option<YamlValue> {
    if let Some(value) = read_date_only_flexible(text) {
        return Some(value);
    }
    match read_timestamp(text) {
        TimestampRead::Date(days) => Some(YamlValue::Date(days)),
        TimestampRead::Naive(micros) => Some(YamlValue::NaiveDateTime(micros)),
        TimestampRead::Aware(micros, offset) => Some(YamlValue::AwareDateTime(micros, offset)),
        _ => None,
    }
}

/// The date alone form under the timestamp tag, its month
/// and day fields standing at one or two digits.
fn read_date_only_flexible(text: &str) -> Option<YamlValue> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut fields: Vec<i64> = Vec::new();
    for field in 0..3 {
        let max = if field == 0 { 4 } else { 2 };
        let mut value: i64 = 0;
        let mut count = 0usize;
        while index < bytes.len() && bytes[index].is_ascii_digit() && count < max {
            value = value * 10 + (bytes[index] - b'0') as i64;
            index += 1;
            count += 1;
        }
        if count == 0 || (field == 0 && count != 4) {
            return None;
        }
        fields.push(value);
        if field < 2 {
            if bytes.get(index) != Some(&b'-') {
                return None;
            }
            index += 1;
        }
    }
    if index != bytes.len() {
        return None;
    }
    let (year, month, day) = (fields[0], fields[1], fields[2]);
    if !(1..=9999).contains(&year) || !valid_calendar_date(year, month, day) {
        return None;
    }
    Some(YamlValue::Date(days_from_civil(year, month, day)))
}

/// Convert a scalar text under a scalar tag, the constructor
/// of the runtime standing in place of plain resolution. A
/// conversion the constructor cannot make refuses.
/// Decode a scalar's text as the runtime's binary
/// constructor decodes it, the base64 decoding the standard
/// library's decodebytes performs. A character outside the
/// ASCII range refuses the whole reading, as the constructor
/// refuses it, and every other character outside the base64
/// alphabet and the padding mark is discarded before the
/// decoding begins. A padding mark standing where a quantum
/// holds no data character is discarded with them. A padding
/// mark after two data characters stands pending, and a data
/// character arriving behind it takes the place it claimed,
/// while a second padding mark completes the quantum at one
/// byte and ends the decoding, everything behind it ignored.
/// A padding mark after three data characters completes the
/// quantum at two bytes and ends the decoding the same way.
/// A padding mark after one data character refuses, and the
/// text ending on a pending padding mark or on a partial
/// quantum of one, two, or three data characters refuses.
fn decode_binary_text(text: &str) -> Option<Vec<u8>> {
    if !text.is_ascii() {
        return None;
    }
    let value_of = |byte: u8| -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a') as u32 + 26),
            b'0'..=b'9' => Some((byte - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out: Vec<u8> = Vec::new();
    let mut quad: Vec<u32> = Vec::new();
    let mut pending_pad = false;
    let mut done = false;
    for byte in text.bytes() {
        if done {
            break;
        }
        if byte == b'=' {
            match quad.len() {
                0 => {}
                1 => return None,
                2 => {
                    if pending_pad {
                        out.push(((quad[0] << 2) | (quad[1] >> 4)) as u8);
                        done = true;
                    } else {
                        pending_pad = true;
                    }
                }
                3 => {
                    out.push(((quad[0] << 2) | (quad[1] >> 4)) as u8);
                    out.push((((quad[1] & 0xf) << 4) | (quad[2] >> 2)) as u8);
                    done = true;
                }
                _ => return None,
            }
        } else if let Some(value) = value_of(byte) {
            if pending_pad {
                pending_pad = false;
            }
            quad.push(value);
            if quad.len() == 4 {
                out.push(((quad[0] << 2) | (quad[1] >> 4)) as u8);
                out.push((((quad[1] & 0xf) << 4) | (quad[2] >> 2)) as u8);
                out.push((((quad[2] & 0x3) << 6) | quad[3]) as u8);
                quad.clear();
            }
        }
    }
    if done {
        return Some(out);
    }
    if pending_pad || !quad.is_empty() {
        return None;
    }
    Some(out)
}

fn convert_tagged_scalar(tag: &Tag, text: &str) -> Option<YamlValue> {
    match tag {
        Tag::Str => Some(YamlValue::Text(text.to_string())),
        Tag::Int => convert_int(text),
        Tag::Float => convert_float(text),
        Tag::Bool => match text.to_lowercase().as_str() {
            "yes" | "true" | "on" => Some(YamlValue::Bool(true)),
            "no" | "false" | "off" => Some(YamlValue::Bool(false)),
            _ => None,
        },
        Tag::Null => Some(YamlValue::Null),
        Tag::Timestamp => convert_timestamp(text),
        Tag::Binary => decode_binary_text(text).map(YamlValue::Bytes),
        _ => None,
    }
}

/// Apply a node's tag to its built value, a scalar tag
/// converting the scalar text and a collection tag standing
/// only on a collection of its own kind.
fn apply_tag(tag: &Tag, built: Built) -> Option<Built> {
    match tag {
        Tag::None | Tag::NonSpecific => Some(built),
        Tag::Seq => {
            if matches!(built.value, YamlValue::List(_)) {
                Some(built)
            } else {
                None
            }
        }
        Tag::Map => {
            if matches!(built.value, YamlValue::Mapping(_)) {
                Some(built)
            } else {
                None
            }
        }
        Tag::Set => {
            // The mapping the node built hands its keys over
            // as the set's members, its merges already
            // expanded and its repeated keys already standing
            // at their first positions under the one key, so
            // the members stand in the node's order with the
            // runtime's equality already told.
            if let YamlValue::Mapping(pairs) = built.value {
                let members: Vec<YamlKey> = pairs.into_iter().map(|(key, _value)| key).collect();
                Some(Built {
                    value: YamlValue::Set(members),
                    tally: built.tally,
                    items: None,
                    text: None,
                    plain_token: None,
                    tag_done: true,
                })
            } else {
                None
            }
        }
        Tag::Omap | Tag::Pairs => {
            // Each entry of the sequence stands as one pair
            // whose key and value are the one pair of the
            // mapping the entry carries, any other entry
            // refusing the reading whole.
            if let YamlValue::List(items) = built.value {
                let mut pairs: Vec<(YamlKey, YamlValue)> = Vec::new();
                for item in items {
                    match item {
                        YamlValue::Mapping(mut entry) if entry.len() == 1 => {
                            pairs.push(entry.pop().expect("one pair"));
                        }
                        _ => return None,
                    }
                }
                Some(Built {
                    value: YamlValue::Pairs(pairs),
                    tally: built.tally,
                    items: None,
                    text: None,
                    plain_token: None,
                    tag_done: true,
                })
            } else {
                None
            }
        }
        Tag::Merge => {
            // The merge tag is a key's tag, and the one
            // node it settles is the plain merge spelling
            // itself, which the flow plain reading stands
            // carrying its plain token for the consumers
            // that can take it as a key to read. The
            // settlement hands that node over as it stands,
            // and every other node under the tag refuses
            // here as it refuses today.
            if built.plain_token.as_deref() == Some("<<") {
                Some(built)
            } else {
                None
            }
        }
        _ if tag.is_constructor() => {
            if built.tag_done {
                return Some(built);
            }
            let text = built.text.clone()?;
            let value = convert_tagged_scalar(tag, &text)?;
            Some(Built {
                value,
                tally: built.tally,
                items: None,
                text: None,
                plain_token: None,
                tag_done: true,
            })
        }
        _ => None,
    }
}

/// Settle a node's properties on its built value, the tag
/// applied and the anchor bound to what the node reads as.
fn finish_props(state: &mut ReadState, props: Props, built: Built) -> Option<Built> {
    let built = apply_tag(&props.tag, built)?;
    if let Some(name) = &props.anchor {
        state.bind(name, &built);
    }
    Some(built)
}

/// The char spans of the lines standing inside block scalar
/// content, as a pre-scan over the text finds them, so the tab
/// standing can exempt the tabs that stand inside block
/// content after the line's space indent. The scan tracks the
/// header grammar of PORT3-1 in outline and the indent rules
/// of PORT3-2 and PORT3-3, and it errs on the side of a
/// narrower span, because a tab it fails to exempt refuses in
/// the tab pass while a tab it wrongly exempts still meets the
/// readers, which refuse tabs in their own positions.
fn block_content_spans(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut starts: Vec<usize> = vec![0];
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '\n' || *ch == '\u{2028}' || *ch == '\u{2029}' {
            starts.push(index + 1);
        }
    }
    let line_end = |li: usize| -> usize {
        if li + 1 < starts.len() {
            starts[li + 1] - 1
        } else {
            chars.len()
        }
    };
    let space_indent = |li: usize| -> usize {
        let mut count = 0;
        let mut index = starts[li];
        while index < chars.len() && chars[index] == ' ' {
            count += 1;
            index += 1;
        }
        count
    };
    // A line standing as blank here is a line of spaces
    // alone. A line carrying a tab after its spaces is a
    // content line carrying the tab, as the reader reads it,
    // so it decides the block indent or ends the block like
    // any other content line and its tab stands in the span.
    let blankish =
        |li: usize| -> bool { (starts[li]..line_end(li)).all(|index| chars[index] == ' ') };
    // Find a block head on a line, as the char index of its
    // style mark, its parsed header, and the indent of the
    // node holding the scalar, or nothing when no head stands.
    let find_head = |li: usize| -> Option<(usize, Option<usize>, usize)> {
        let start = starts[li];
        let end = line_end(li);
        #[derive(PartialEq)]
        enum St {
            Normal,
            Single,
            Double,
            Comment,
        }
        let mut state = St::Normal;
        let mut index = start;
        let mut previous: Option<char> = None;
        while index < end {
            let ch = chars[index];
            match state {
                St::Normal => match ch {
                    '#' if matches!(previous, None | Some(' ')) => state = St::Comment,
                    '\'' => state = St::Single,
                    '"' => state = St::Double,
                    '|' | '>' => {
                        // The header grammar, in outline.
                        let mut cursor = index + 1;
                        let mut chomping = false;
                        let mut digit: Option<usize> = None;
                        let mut valid = true;
                        while cursor < end {
                            match chars[cursor] {
                                '+' | '-' if !chomping => {
                                    chomping = true;
                                    cursor += 1;
                                }
                                '1'..='9' if digit.is_none() => {
                                    digit = Some(chars[cursor] as usize - '0' as usize);
                                    cursor += 1;
                                }
                                _ => break,
                            }
                        }
                        while cursor < end && chars[cursor] == ' ' {
                            cursor += 1;
                        }
                        if cursor < end && chars[cursor] != '#' {
                            valid = false;
                        }
                        if !valid {
                            return None;
                        }
                        // The holding node's indent. A head
                        // behind a key takes the key's indent,
                        // a head behind a dash takes the dash
                        // line's indent, and a head standing
                        // alone at the root takes 0.
                        let prefix: String = chars[start..index].iter().collect();
                        let mut trimmed = prefix.trim_end().to_string();
                        // Properties standing behind the head
                        // are cut from the prefix, a head
                        // standing behind a property standing
                        // as a head under PORT4-8.
                        loop {
                            let cut = match trimmed.rfind(' ') {
                                Some(space) => {
                                    let token = &trimmed[space + 1..];
                                    if token.starts_with('&') || token.starts_with('!') {
                                        Some(space)
                                    } else {
                                        None
                                    }
                                }
                                None => {
                                    if trimmed.starts_with('&') || trimmed.starts_with('!') {
                                        Some(0)
                                    } else {
                                        None
                                    }
                                }
                            };
                            match cut {
                                Some(at) => {
                                    trimmed.truncate(at);
                                    let standing = trimmed.trim_end().to_string();
                                    trimmed = standing;
                                }
                                None => break,
                            }
                        }
                        let trimmed = trimmed.as_str();
                        let parent = if trimmed.is_empty() || trimmed == "---" {
                            0
                        } else if trimmed == "-" || trimmed.ends_with(" -") {
                            space_indent(li)
                        } else if trimmed.ends_with(':') {
                            let mut key_start = space_indent(li);
                            let body = &chars[start + key_start..index];
                            if body.first() == Some(&'-') {
                                let mut walk = 1;
                                while walk < body.len() && body[walk] == ' ' {
                                    walk += 1;
                                }
                                key_start += walk;
                            }
                            key_start
                        } else {
                            return None;
                        };
                        return Some((index, digit, parent));
                    }
                    _ => {}
                },
                St::Comment => {}
                St::Single => {
                    if ch == '\'' {
                        state = St::Normal;
                    }
                }
                St::Double => match ch {
                    '\\' => {
                        index += 1;
                    }
                    '"' => state = St::Normal,
                    _ => {}
                },
            }
            previous = Some(ch);
            index += 1;
        }
        None
    };
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut li = 0;
    while li < starts.len() {
        let Some((_, digit, parent)) = find_head(li) else {
            li += 1;
            continue;
        };
        let mut block_indent: Option<usize> = digit.map(|digit| parent + digit);
        let mut floor = 0usize;
        let mut scan = li + 1;
        let mut content: Vec<usize> = Vec::new();
        let mut pending: Vec<usize> = Vec::new();
        let mut decided = block_indent.is_some();
        while scan < starts.len() {
            if blankish(scan) {
                let spaces = space_indent(scan);
                if !decided && spaces > floor {
                    floor = spaces;
                }
                pending.push(scan);
                scan += 1;
                continue;
            }
            let indent = space_indent(scan);
            match block_indent {
                None => {
                    if indent <= parent {
                        break;
                    }
                    if indent < floor {
                        break;
                    }
                    block_indent = Some(indent);
                    decided = true;
                    content.append(&mut pending);
                    content.push(scan);
                    scan += 1;
                }
                Some(width) => {
                    if indent < width {
                        break;
                    }
                    content.append(&mut pending);
                    content.push(scan);
                    scan += 1;
                }
            }
        }
        // Trailing blank lines stand inside the span only
        // where a tab stands on them past the block indent.
        if let Some(width) = block_indent {
            for pending_line in pending {
                if space_indent(pending_line) >= width {
                    content.push(pending_line);
                }
            }
        }
        for line in content {
            spans.push((starts[line], line_end(line)));
        }
        li = if scan > li { scan } else { li + 1 };
    }
    spans
}

/// Whether a tab stands anywhere the runtime scanner refuses
/// one, which is everywhere outside a quoted scalar and a
/// comment and block scalar content, as the tab probe
/// recorded.
fn tab_refused(text: &str) -> bool {
    let spans = block_content_spans(text);
    let in_span = |index: usize| spans.iter().any(|(lo, hi)| index >= *lo && index < *hi);
    #[derive(PartialEq)]
    enum State {
        Normal,
        Single,
        Double,
        Comment,
    }
    let mut state = State::Normal;
    let mut previous: Option<char> = None;
    let mut chars = text.chars().enumerate().peekable();
    while let Some((index, ch)) = chars.next() {
        // Characters standing inside block content are
        // opaque to this scan. A quote or a hash there is
        // content, never the opening of a quoted scalar or
        // a comment, so it must not move the state, and a
        // tab there is exempt by the span itself.
        if in_span(index) {
            previous = Some(ch);
            continue;
        }
        match state {
            State::Normal => match ch {
                '\t' => {
                    if !in_span(index) {
                        return true;
                    }
                }
                '\n' => {}
                '#' if matches!(
                    previous,
                    None | Some(' ' | '\n' | '\u{2028}' | '\u{2029}' | ',' | '[' | '{')
                ) =>
                {
                    state = State::Comment;
                }
                '\'' if matches!(
                    previous,
                    None | Some(
                        ' ' | '\n' | '\u{2028}' | '\u{2029}' | ':' | ',' | '[' | '{' | '-' | '?'
                    )
                ) =>
                {
                    state = State::Single;
                }
                '"' if matches!(
                    previous,
                    None | Some(
                        ' ' | '\n' | '\u{2028}' | '\u{2029}' | ':' | ',' | '[' | '{' | '-' | '?'
                    )
                ) =>
                {
                    state = State::Double;
                }
                _ => {}
            },
            State::Comment => {
                if ch == '\n' || ch == '\u{2028}' || ch == '\u{2029}' {
                    state = State::Normal;
                }
            }
            State::Single => {
                if ch == '\'' {
                    if matches!(chars.peek(), Some((_, '\''))) {
                        chars.next();
                        previous = Some('\'');
                        continue;
                    }
                    state = State::Normal;
                }
            }
            State::Double => match ch {
                '\\' => {
                    chars.next();
                }
                '"' => state = State::Normal,
                _ => {}
            },
        }
        previous = Some(ch);
    }
    false
}

/// Whether a directive line stands in a shape the runtime
/// reading accepts, as the directive probes recorded. A version
/// directive names a major version of one, written as whole
/// digits on both sides of its point, and stands once in a
/// document. A tag directive carries a handle opening with the
/// tag mark and a prefix, and a handle stands once. Any other
/// directive stands as it stands and is passed over. A comment
/// may close the line, and a tab in the directive itself
/// refuses it.
fn directive_stands(
    text: &str,
    version_seen: &mut bool,
    tag_handles: &mut Vec<(String, String)>,
) -> bool {
    let head = match text.find(" #") {
        Some(index) => &text[..index],
        None => text,
    };
    if head.contains('\t') {
        return false;
    }
    let mut tokens = head[1..].split(' ').filter(|token| !token.is_empty());
    let Some(name) = tokens.next() else {
        return false;
    };
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return false;
    }
    let args: Vec<&str> = tokens.collect();
    match name {
        "YAML" => {
            if *version_seen || args.len() != 1 {
                return false;
            }
            let Some((major, minor)) = args[0].split_once('.') else {
                return false;
            };
            let digits =
                |part: &str| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit());
            if !digits(major) || !digits(minor) {
                return false;
            }
            if major.trim_start_matches('0') != "1" {
                return false;
            }
            *version_seen = true;
            true
        }
        "TAG" => {
            if args.len() != 2 || !args[0].starts_with('!') {
                return false;
            }
            if tag_handles.iter().any(|(standing, _)| standing == args[0]) {
                return false;
            }
            tag_handles.push((args[0].to_string(), args[1].to_string()));
            true
        }
        _ => true,
    }
}

/// Read a YAML document the way the safe load of the runtime
/// reads it over the subset this reader covers, or refuse it.
/// A byte order mark at the head of the text is cut first, as
/// the runtime reading cuts it. The reading itself runs on a
/// spawned thread carrying a stack sized for the nesting bound,
/// so a document nested to the bound reads or refuses on its
/// merits on every surface and the bound never fires as an
/// abort. A panic inside the reading resumes across the join,
/// and a spawn that fails falls back to reading in place.
pub fn parse_yaml(text: &str) -> Option<YamlValue> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let owned = text.to_string();
    let spawned = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || parse_yaml_document(&owned));
    match spawned {
        Ok(handle) => match handle.join() {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        },
        Err(_) => parse_yaml_document(text),
    }
}

/// Whether the words standing behind a start marker on its
/// own line open a block node, a block mapping pair or a
/// block sequence entry, the forms the runtime reading
/// refuses there (PORT8-1). Properties standing in front of
/// the node are read past, as the reading reads them, and a
/// pair standing inside a flow collection is flow content
/// and no block pair.
fn inline_opens_block_node(rest: &str) -> bool {
    let mut text = rest.trim_start();
    loop {
        if let Some(after) = text.strip_prefix('&') {
            match after.split_once(' ') {
                Some((_, tail)) => text = tail.trim_start(),
                None => return false,
            }
        } else if text.starts_with('!') {
            match text.split_once(' ') {
                Some((_, tail)) => text = tail.trim_start(),
                None => return false,
            }
        } else {
            break;
        }
    }
    if text == "-" || text.starts_with("- ") {
        return true;
    }
    if text.starts_with('[') || text.starts_with('{') {
        return false;
    }
    if let Some(quote) = text.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let mut index = 1;
        let chars: Vec<char> = text.chars().collect();
        while index < chars.len() {
            if quote == '"' && chars[index] == '\\' {
                index += 2;
                continue;
            }
            if chars[index] == quote {
                if quote == '\'' && chars.get(index + 1) == Some(&'\'') {
                    index += 2;
                    continue;
                }
                break;
            }
            index += 1;
        }
        if index >= chars.len() {
            return false;
        }
        let after: String = chars[index + 1..].iter().collect();
        let after = after.trim_start();
        return after == ":" || after.starts_with(": ");
    }
    let scalar = match text.find(" #") {
        Some(at) => &text[..at],
        None => text,
    };
    let scalar = scalar.trim_end();
    scalar.ends_with(':') || scalar.contains(": ")
}

fn parse_yaml_document(text: &str) -> Option<YamlValue> {
    // The runtime reading takes the carriage return forms and
    // the next line character all as line breaks, and those
    // forms fold to the one line feed before the text is split
    // into lines. The line separator and the paragraph
    // separator stand in the text as themselves, as PORT15-1
    // states, and the split below cuts at every break standing
    // in the text, at a line feed and at either separator, and
    // records on each line the kind of the break that stood
    // behind it, the last line carrying no break. The tab
    // standing is taken over the text as it stands after that
    // folding, so the block content spans the tab pass exempts
    // are found under every break form, the carriage return
    // forms among them, while the printable screen below runs
    // over the text as given.
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{85}', "\n");
    if tab_refused(&normalized) {
        return None;
    }
    for ch in text.chars() {
        let code = ch as u32;
        let printable = ch == '\t'
            || ch == '\n'
            || ch == '\r'
            || (0x20..=0x7E).contains(&code)
            || code == 0x85
            || (0xA0..=0xD7FF).contains(&code)
            || (0xE000..=0xFFFD).contains(&code)
            || (0x10000..=0x10FFFF).contains(&code);
        if !printable {
            return None;
        }
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut segments: Vec<(&str, Option<BreakKind>)> = Vec::new();
    let mut segment_start = 0usize;
    for (index, ch) in normalized.char_indices() {
        let kind = match ch {
            '\n' => Some(BreakKind::LineFeed),
            '\u{2028}' => Some(BreakKind::LineSeparator),
            '\u{2029}' => Some(BreakKind::ParagraphSeparator),
            _ => None,
        };
        if let Some(kind) = kind {
            segments.push((&normalized[segment_start..index], Some(kind)));
            segment_start = index + ch.len_utf8();
        }
    }
    segments.push((&normalized[segment_start..], None));
    for (raw, kind) in &segments {
        let raw = *raw;
        let mut indent = 0usize;
        let mut content_start = raw.len();
        for (position, ch) in raw.char_indices() {
            match ch {
                ' ' => indent += 1,
                _ => {
                    content_start = position;
                    break;
                }
            }
        }
        let content = &raw[content_start..];
        lines.push(Line {
            indent,
            text: content.to_string(),
            raw: raw.to_string(),
            has_break: kind.is_some(),
            kind: kind.unwrap_or(BreakKind::LineFeed),
        });
    }
    let mut cursor = Cursor { lines, pos: 0 };
    // Directives and one opener may stand in front of the
    // document, and one closer may stand behind it. Anything
    // further of the document apparatus refuses the reading.
    // A directive stands only in the shapes the runtime reading
    // accepts, and a document carrying any directive opens with
    // the document marker, as the directive probes recorded.
    let Some(mut start) = cursor.peek_content() else {
        return Some(YamlValue::Null);
    };
    let mut directives = 0usize;
    let mut opener_seen = false;
    let mut version_seen = false;
    let mut tag_handles: Vec<(String, String)> = Vec::new();
    loop {
        let text = cursor.lines[start].text.clone();
        if text.starts_with('%') {
            if !directive_stands(&text, &mut version_seen, &mut tag_handles) {
                return None;
            }
            directives += 1;
            start = cursor.next_content(start + 1)?;
            continue;
        }
        if text == "---" {
            opener_seen = true;
            start = cursor.next_content(start + 1).unwrap_or(cursor.lines.len());
        } else if let Some(rest) = text.strip_prefix("--- ") {
            // The runtime reading takes the words behind the
            // marker as the document's one node, so a block
            // mapping pair or a block sequence entry standing
            // there refuses the reading, in the plain form and
            // behind a property and with a quoted key alike,
            // where a scalar or a flow node standing there
            // reads as the document (PORT8-1).
            if inline_opens_block_node(rest) {
                return None;
            }
            opener_seen = true;
            let standing = cursor.lines[start].clone();
            cursor.lines[start] = Line {
                indent: 0,
                text: rest.to_string(),
                raw: rest.to_string(),
                has_break: standing.has_break,
                kind: standing.kind,
            };
        }
        break;
    }
    if directives > 0 && !opener_seen {
        return None;
    }
    let mut end = cursor.lines.len();
    {
        let mut index = start;
        while let Some(found) = cursor.next_content(index) {
            let text = cursor.lines[found].text.clone();
            let column_zero = cursor.lines[found].indent == 0;
            if column_zero && (text == "---" || text.starts_with("--- ")) {
                return None;
            }
            if column_zero && (text == "..." || text.starts_with("... ")) {
                // An end marker with no document in front of
                // it refuses the reading, as the runtime
                // reading refuses it. A start marker standing
                // before it opens the empty document it ends,
                // and content standing before it is the
                // document it ends (PORT8-2).
                if found == start && !opener_seen {
                    return None;
                }
                // A document end marker may carry a comment
                // behind it and still end the document, as the
                // runtime reading takes it. Any other words
                // behind the marker refuse the reading.
                let tail = if text == "..." { "" } else { &text[4..] };
                let tail = tail.trim();
                if !(tail.is_empty() || tail.starts_with('#')) {
                    return None;
                }
                if cursor.next_content(found + 1).is_some() {
                    return None;
                }
                end = found;
                break;
            }
            index = found + 1;
        }
    }
    cursor.lines.truncate(end);
    cursor.pos = start;
    if cursor.peek_content().is_none() {
        return Some(YamlValue::Null);
    }
    let mut bindings: HashMap<String, String> = HashMap::new();
    bindings.insert("!".to_string(), "!".to_string());
    bindings.insert("!!".to_string(), "tag:yaml.org,2002:".to_string());
    for (handle, prefix) in &tag_handles {
        bindings.insert(handle.clone(), prefix.clone());
    }
    let mut state = ReadState {
        bindings,
        bound: HashMap::new(),
        building: HashSet::new(),
        expansions: 0,
    };
    let value = parse_block(&mut cursor, 0, 0, &mut state)?.value;
    if cursor.peek_content().is_some() {
        return None;
    }
    if !keys_stand_hashable(&value) {
        return None;
    }
    Some(value)
}

/// A scalar key, the kind a mapping's keys and a set's
/// members must stand as.
fn key_is_scalar(key: &YamlKey) -> bool {
    matches!(
        key,
        YamlKey::Null
            | YamlKey::Bool(_)
            | YamlKey::Int(_)
            | YamlKey::BigInt(_)
            | YamlKey::Float(_)
            | YamlKey::Text(_)
            | YamlKey::Date(_)
            | YamlKey::NaiveDateTime(_)
            | YamlKey::AwareDateTime(_, _)
            | YamlKey::Bytes(_)
    )
}

/// The border PORT10-2 states, walked over the built
/// document. The runtime reading builds a mapping or a set
/// only from keys it can hash, so a container standing as a
/// mapping's key or as a set's member refuses the reading
/// whole, while a list of pairs builds neither and its keys
/// stand as any value. The reader builds all three standings
/// from the one key model, so the border is held here, over
/// the finished value: every mapping's keys and every set's
/// members must stand as scalar keys, and a pair's key is
/// walked as the value it stands as, the mappings and sets
/// inside it held to the same border.
fn keys_stand_hashable(value: &YamlValue) -> bool {
    match value {
        YamlValue::Mapping(pairs) => pairs
            .iter()
            .all(|(key, item)| key_is_scalar(key) && keys_stand_hashable(item)),
        YamlValue::Set(members) => members.iter().all(key_is_scalar),
        YamlValue::Pairs(pairs) => pairs.iter().all(|(key, item)| {
            keys_stand_hashable(&key_to_value(key)) && keys_stand_hashable(item)
        }),
        YamlValue::List(items) => items.iter().all(keys_stand_hashable),
        _ => true,
    }
}

/// Read the block standing on the following lines under a
/// tag the caller holds, the tag reaching a bare scalar node
/// standing there so the scalar converts from its own text
/// under the tag, and every other node shape reading exactly
/// as parse_block reads it, the tag settling on the built
/// node in the caller's finish.
fn parse_block_tagged(
    cursor: &mut Cursor,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
    tag: &Tag,
) -> Option<Built> {
    let li = cursor.peek_content()?;
    let line = cursor.lines[li].clone();
    if line.text.starts_with('\t') {
        return None;
    }
    let bare_scalar = matches!(
        split_properties(&line.text, &state.bindings),
        PropSplit::None
    ) && !is_dash_line(&line)
        && !mapping_shape(&line.text, &state.bindings);
    if !bare_scalar {
        return parse_block(cursor, parent_indent, depth, state);
    }
    cursor.pos = li + 1;
    parse_inline_node(
        &line.text,
        cursor,
        line.indent,
        parent_indent,
        depth,
        state,
        tag,
    )
}

fn parse_block(
    cursor: &mut Cursor,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<Built> {
    if depth > MAX_YAML_NESTING {
        return None;
    }
    let li = cursor.peek_content()?;
    let line = cursor.lines[li].clone();
    if line.text.starts_with('\t') {
        return None;
    }
    match split_properties(&line.text, &state.bindings) {
        PropSplit::Refuse => return None,
        PropSplit::Props(props, offset) => {
            let rest = &line.text[offset..];
            if rest.is_empty() || rest.starts_with('#') {
                // Properties standing alone on the node's line
                // bind the block standing on the following
                // lines, or the empty scalar when no block
                // stands behind them.
                cursor.pos = li + 1;
                if let Some(name) = &props.anchor {
                    state.mark(name)?;
                }
                let built =
                    read_following_block(cursor, &line, parent_indent, depth, state, &props.tag)?;
                return finish_props(state, props, built);
            }
        }
        PropSplit::None => {}
    }
    if is_dash_line(&line) {
        parse_sequence(cursor, line.indent, depth, state)
    } else if mapping_shape(&line.text, &state.bindings) {
        parse_mapping(cursor, line.indent, depth, state)
    } else {
        cursor.pos = li + 1;
        parse_inline_value(&line.text, cursor, line.indent, parent_indent, depth, state)
    }
}

/// Read the node standing on the lines behind a line that
/// carried properties alone, a dash standing at the line's
/// own indent opening its sequence and a block standing at
/// or deeper than the line's indent opening under the
/// parent, the empty scalar standing when no node stands
/// behind.
fn read_following_block(
    cursor: &mut Cursor,
    line: &Line,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
    tag: &Tag,
) -> Option<Built> {
    let Some(nj) = cursor.peek_content() else {
        return Some(Built::empty_scalar());
    };
    let next = cursor.lines[nj].clone();
    if next.indent == line.indent && is_dash_line(&next) {
        parse_sequence(cursor, next.indent, depth + 1, state)
    } else if next.indent >= line.indent || next.indent > parent_indent {
        parse_block_tagged(cursor, parent_indent, depth + 1, state, tag)
    } else {
        Some(Built::empty_scalar())
    }
}

/// Read a mapping value standing on the lines behind its key
/// line, the block standing deeper or a sequence standing at
/// the key's own indent, the empty scalar standing when no
/// value stands behind.
fn following_block_value(
    cursor: &mut Cursor,
    indent: usize,
    depth: usize,
    state: &mut ReadState,
    tag: &Tag,
) -> Option<Built> {
    match cursor.peek_content() {
        None => Some(Built::empty_scalar()),
        Some(nj) => {
            let next = cursor.lines[nj].clone();
            if next.indent > indent {
                parse_block_tagged(cursor, indent, depth + 1, state, tag)
            } else if next.indent == indent && is_dash_line(&next) {
                parse_sequence(cursor, indent, depth + 1, state)
            } else {
                Some(Built::empty_scalar())
            }
        }
    }
}

/// Whether a line's text opens the value half of an explicit
/// pair, the colon standing alone on the line or in front of
/// a space.
fn colon_opening(text: &str) -> bool {
    text == ":" || text.starts_with(": ")
}

/// Read the key node standing on an explicit pair's question
/// mark line, under the standings of key position. An anchor
/// standing on the key refuses, and a tag stands only as the
/// merge tag or the non-specific tag, and an alias refuses.
/// The node itself reads as the standing key path reads it, a
/// plain or quoted scalar through the key scalar reading and
/// a block scalar or a flow node through the inline reading,
/// a collection standing as the key refusing as the runtime's
/// unhashable key dies. Hands back the key and whether the
/// pair merges, the merge standing on the merge tag or on the
/// plain merge spelling standing as the node's own text.
/// Read a block collection standing as an explicit key, a
/// sequence or a mapping whose first line stands on the
/// question mark line behind the mark. The collection's
/// first line is laid into the line table at the column its
/// first token stands at, the question line's own indent
/// plus the raw key text's offset in it, and the block
/// reading runs over it and the lines behind it as it runs
/// in value position, the question line standing restored
/// behind the cursor when the reading is done.
fn read_collection_key(
    raw: &str,
    text: &str,
    cursor: &mut Cursor,
    indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<Built> {
    let key_line_index = cursor.pos.checked_sub(1)?;
    let original = cursor.lines[key_line_index].clone();
    let column = original.indent + (original.text.len() - raw.len());
    cursor.lines[key_line_index] = Line {
        indent: column,
        text: text.to_string(),
        raw: text.to_string(),
        has_break: original.has_break,
        kind: original.kind,
    };
    cursor.pos = key_line_index;
    let built = parse_block(cursor, indent, depth, state);
    cursor.lines[key_line_index] = original;
    built
}

fn read_explicit_key_text(
    text: &str,
    cursor: &mut Cursor,
    indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<(YamlKey, bool)> {
    let (key_props, key_offset) = match split_key_properties(text, &state.bindings) {
        KeySplit::Refuse => return None,
        KeySplit::None => (
            KeyProps {
                anchor: None,
                tag: KeyTag::None,
            },
            0,
        ),
        KeySplit::Props(props, offset) => (props, offset),
    };
    // A collection tag standing in front of a collection
    // key is the key node's own tag, as it is in the block
    // mapping's compact reading: the original tag is kept
    // beside the key properties for the branches that read
    // a collection, and the branches that read a scalar
    // still refuse under the class as they refuse today.
    let mut collection_tag: Option<Tag> = None;
    if key_props.tag == KeyTag::Refuse {
        match split_properties(text, &state.bindings) {
            PropSplit::Props(props, _)
                if matches!(
                    props.tag,
                    Tag::Seq | Tag::Map | Tag::Set | Tag::Omap | Tag::Pairs
                ) =>
            {
                collection_tag = Some(props.tag.clone());
            }
            _ => return None,
        }
    }
    let merge_tag = key_props.tag == KeyTag::Merge;
    let raw = text[key_offset..].trim_start();
    let first = raw.chars().next()?;
    if first == '*' {
        // An alias standing as the key, a property standing
        // in front of an alias refusing as it refuses in
        // value position.
        if key_props.anchor.is_some() || key_props.tag != KeyTag::None {
            return None;
        }
        let name: String = raw[1..]
            .chars()
            .take_while(|ch| is_anchor_char_char(*ch))
            .collect();
        if name.is_empty() {
            return None;
        }
        check_trailing(&raw[1 + name.len()..])?;
        return Some(read_alias_key(state, &name)?);
    }
    let value = if first == '"' || first == '\'' {
        if collection_tag.is_some() {
            return None;
        }
        let (quoted, rest) = read_block_quoted(raw, cursor, first == '"')?;
        check_trailing(&rest)?;
        match &key_props.tag {
            KeyTag::Convert(tag) => convert_tagged_scalar(tag, &quoted)?,
            _ => YamlValue::Text(quoted),
        }
    } else if first == '|' || first == '>' || first == '[' || first == '{' {
        let built = parse_inline_value(raw, cursor, indent, indent, depth, state)?;
        match &key_props.tag {
            KeyTag::Convert(tag) => apply_tag(tag, built)?.value,
            _ => match &collection_tag {
                Some(tag) => apply_tag(tag, built)?.value,
                None => built.value,
            },
        }
    } else {
        let plain_text = cut_comment(raw).trim();
        if plain_text == "-"
            || plain_text.starts_with("- ")
            || mapping_shape(plain_text, &state.bindings)
        {
            // A dash opening or a mapping standing on the
            // question mark line opens a collection as the
            // key node. The collection builds as it builds
            // in value position, and whether it may stand
            // as this mapping's key is the border the reader
            // holds over the built document, a pair's key
            // carrying a collection where an ordinary
            // mapping's key refuses it.
            let built = read_collection_key(raw, plain_text, cursor, indent, depth, state)?;
            let key_value = match &key_props.tag {
                KeyTag::Convert(tag) => apply_tag(tag, built)?.value,
                _ => match &collection_tag {
                    Some(tag) => apply_tag(tag, built)?.value,
                    None => built.value,
                },
            };
            return Some((YamlKey::of(&key_value)?, false));
        }
        if collection_tag.is_some() {
            return None;
        }
        // A separator ending the line the key stands on
        // gathers the key's continuation standing on the
        // following lines, the chunks folding by the run
        // rule as a block plain scalar folds, the gathering
        // stopping at the pair's colon line and at a line
        // carrying a mapping pair of its own. A key whose
        // line ends at a line feed keeps the standing
        // reading whole.
        let gathered = gather_explicit_plain_key(cursor, indent, plain_text, &state.bindings);
        let plain_text: &str = match &gathered {
            Some(text) => text.as_str(),
            None => plain_text,
        };
        match &key_props.tag {
            KeyTag::Convert(tag) => convert_tagged_scalar(tag, plain_text)?,
            _ => parse_key_scalar(plain_text)?,
        }
    };
    let plain_merge = merge_tag
        || (matches!(key_props.tag, KeyTag::None | KeyTag::NonSpecific)
            && first != '"'
            && first != '\''
            && cut_comment(raw).trim() == "<<");
    if let Some(name) = &key_props.anchor {
        bind_key_anchor(state, name, &value, plain_merge)?;
    }
    let key = YamlKey::of(&value)?;
    Some((key, plain_merge))
}

/// Gather the continuation of an explicit plain key whose
/// first line ends at a separator, folding the chunks by
/// the run rule over the breaks standing between them.
/// The gathering walks the lines behind the key's own line
/// while they stand deeper than the mapping and are not
/// the pair's colon line and do not carry a mapping pair
/// of their own, a line carrying a pair ending the
/// gathering as the runtime reading refuses the document
/// at that line, blank lines between chunks standing in
/// the run as they stand in a block plain scalar, and a
/// comment line ends the gathering as it ends a plain
/// scalar. Hands back nothing when the key stands on one
/// line, and advances the cursor past the gathered lines
/// when it hands a folded key back.
fn gather_explicit_plain_key(
    cursor: &mut Cursor,
    indent: usize,
    first_text: &str,
    bindings: &HashMap<String, String>,
) -> Option<String> {
    let key_line = cursor.pos.checked_sub(1)?;
    if !cursor.lines[key_line].kind.is_separator() {
        return None;
    }
    let mut chunks: Vec<String> = vec![first_text.to_string()];
    let mut runs: Vec<Vec<BreakKind>> = Vec::new();
    let mut prev = key_line;
    let mut scan = cursor.pos;
    loop {
        let mut run = vec![cursor.lines[prev].kind];
        let mut probe = scan;
        while probe < cursor.lines.len() && cursor.lines[probe].text.trim().is_empty() {
            run.push(cursor.lines[probe].kind);
            probe += 1;
        }
        if probe >= cursor.lines.len() {
            break;
        }
        let next = &cursor.lines[probe];
        if next.text.trim_start().starts_with('#')
            || next.indent <= indent
            || colon_opening(&next.text)
            || mapping_pair_shape(&next.text, bindings)
        {
            break;
        }
        chunks.push(cut_comment(&next.text).trim().to_string());
        runs.push(run);
        prev = probe;
        scan = probe + 1;
    }
    if chunks.len() == 1 {
        return None;
    }
    let mut folded = chunks[0].clone();
    for (index, run) in runs.iter().enumerate() {
        folded.push_str(&fold_break_run(run));
        folded.push_str(&chunks[index + 1]);
    }
    cursor.pos = prev + 1;
    Some(folded)
}

fn parse_mapping(
    cursor: &mut Cursor,
    indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<Built> {
    let mut explicit: Vec<(YamlKey, Built)> = Vec::new();
    let mut merge_sources: Vec<Built> = Vec::new();
    loop {
        let Some(li) = cursor.peek_content() else {
            break;
        };
        let line = cursor.lines[li].clone();
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return None;
        }
        if line.text.starts_with('\t') {
            return None;
        }
        if is_dash_line(&line) {
            break;
        }
        // A property standing in front of the key stands at
        // the node's own start only on the mapping's first
        // pair, and what it can do there is bind the first
        // key under its anchor and carry the tags key
        // position allows. On any later pair the properties
        // stand in key position and refuse beyond those tags.
        let (key_props, key_offset) = match split_key_properties(&line.text, &state.bindings) {
            KeySplit::Refuse => return None,
            KeySplit::None => (
                KeyProps {
                    anchor: None,
                    tag: KeyTag::None,
                },
                0,
            ),
            KeySplit::Props(props, offset) => (props, offset),
        };
        // A collection tag standing in front of a flow
        // collection key is the key node's own tag, not a
        // key conversion, so it stands apart from the tag
        // classes of key position: the original tag is kept
        // beside the key properties for the one branch that
        // reads such a key, and every other branch still
        // refuses under the class as it refuses today, the
        // scalar reading carrying the refusal in its own arm.
        let mut flow_key_tag: Option<Tag> = None;
        if key_props.tag == KeyTag::Refuse {
            match split_properties(&line.text, &state.bindings) {
                PropSplit::Props(props, _)
                    if matches!(
                        props.tag,
                        Tag::Seq | Tag::Map | Tag::Set | Tag::Omap | Tag::Pairs
                    ) =>
                {
                    flow_key_tag = Some(props.tag.clone());
                }
                _ => return None,
            }
        }
        let after_props = &line.text[key_offset..];
        if explicit_opening(after_props) {
            // An explicit pair. The key node stands on the
            // question mark line or on the lines behind a bare
            // mark, and the value stands behind the colon line
            // at the mapping's own indent, a pair whose colon
            // line never stands holding a null value, as the
            // runtime reads it.
            if key_props.anchor.is_some() {
                return None;
            }
            let merge_tag = key_props.tag == KeyTag::Merge;
            let key_text = after_props[1..].trim_start();
            if colon_opening(key_text) {
                return None;
            }
            cursor.pos = li + 1;
            let (key, plain_merge) = if key_text.is_empty() {
                match cursor.peek_content() {
                    Some(nj) if cursor.lines[nj].indent > indent => {
                        // The key node standing on the
                        // following lines stands under the
                        // standings of key position as the key
                        // on the question mark line does.
                        let next_text = cursor.lines[nj].text.clone();
                        let (_next_props, next_offset) =
                            match split_key_properties(&next_text, &state.bindings) {
                                KeySplit::Refuse => return None,
                                KeySplit::None => (
                                    KeyProps {
                                        anchor: None,
                                        tag: KeyTag::None,
                                    },
                                    0,
                                ),
                                KeySplit::Props(props, offset) => (props, offset),
                            };
                        let after_next = next_text[next_offset..].trim_start();
                        if after_next.starts_with('*') && next_offset == 0 {
                            // An alias standing as the key on
                            // the following lines.
                            let name: String = after_next[1..]
                                .chars()
                                .take_while(|ch| is_anchor_char_char(*ch))
                                .collect();
                            if name.is_empty() {
                                return None;
                            }
                            check_trailing(&after_next[1 + name.len()..])?;
                            cursor.pos = nj + 1;
                            read_alias_key(state, &name)?
                        } else if next_offset > 0 {
                            // Properties standing on the key
                            // line send the key through the key
                            // path, where the merge spelling
                            // stands as a key and never meets
                            // the value position refusal.
                            cursor.pos = nj + 1;
                            read_explicit_key_text(&next_text, cursor, indent, depth + 1, state)?
                        } else {
                            let line_merge = after_next.trim() == "<<";
                            let built = parse_block(cursor, indent, depth + 1, state)?;
                            (YamlKey::of(&built.value)?, line_merge)
                        }
                    }
                    _ => (YamlKey::Null, false),
                }
            } else {
                read_explicit_key_text(key_text, cursor, indent, depth + 1, state)?
            };
            let value = match cursor.peek_content() {
                Some(nj)
                    if cursor.lines[nj].indent == indent
                        && colon_opening(&cursor.lines[nj].text) =>
                {
                    let value_line = cursor.lines[nj].clone();
                    cursor.pos = nj + 1;
                    let value_text = value_line.text[1..].trim_start().to_string();
                    if value_text.is_empty() || value_text.starts_with('#') {
                        following_block_value(cursor, indent, depth, state, &Tag::None)?
                    } else {
                        match split_properties(&value_text, &state.bindings) {
                            PropSplit::Refuse => return None,
                            PropSplit::Props(props, offset)
                                if value_text[offset..].is_empty()
                                    || value_text[offset..].starts_with('#') =>
                            {
                                if let Some(name) = &props.anchor {
                                    state.mark(name)?;
                                }
                                let built = following_block_value(
                                    cursor, indent, depth, state, &props.tag,
                                )?;
                                finish_props(state, props, built)?
                            }
                            _ => parse_inline_value(
                                &value_text,
                                cursor,
                                indent,
                                indent,
                                depth + 1,
                                state,
                            )?,
                        }
                    }
                }
                _ => Built::empty_scalar(),
            };
            if merge_tag || plain_merge {
                merge_sources.push(value);
            } else {
                explicit.push((key, value));
            }
            continue;
        }
        let after_props = &line.text[key_offset..];
        let (key, merge_key, rest) = if after_props.trim_start().starts_with('*') {
            // An alias standing as a compact key, a property
            // standing in front of an alias refusing as it
            // refuses in value position.
            if key_props.anchor.is_some() || key_props.tag != KeyTag::None {
                return None;
            }
            let trimmed = after_props.trim_start();
            let name: String = trimmed[1..]
                .chars()
                .take_while(|ch| is_anchor_char_char(*ch))
                .collect();
            if name.is_empty() {
                return None;
            }
            let after_name = trimmed[1 + name.len()..].trim_start();
            if !after_name.starts_with(':') {
                return None;
            }
            if after_name.len() > 1 && !matches!(after_name.as_bytes()[1], b' ' | b'\t') {
                return None;
            }
            let (key, merge_key) = read_alias_key(state, &name)?;
            (key, merge_key, after_name[1..].trim_start().to_string())
        } else if let Some((collection_end, rest_start)) = flow_key_split(after_props) {
            // A flow collection standing as a compact key, the
            // pair's colon standing behind its closing bracket.
            // The key node reads as the flow value it is, under
            // the standings the flow key reading gives a key
            // under a tag, and an anchor in front of it binds
            // the key value as it binds a scalar key.
            let built = parse_inline_value(
                &after_props[..collection_end],
                cursor,
                indent,
                indent,
                depth + 1,
                state,
            )?;
            let key_value = match &key_props.tag {
                KeyTag::Convert(tag) => apply_tag(tag, built)?.value,
                _ => match &flow_key_tag {
                    Some(tag) => apply_tag(tag, built)?.value,
                    None => built.value,
                },
            };
            if let Some(name) = &key_props.anchor {
                bind_key_anchor(state, name, &key_value, false)?;
            }
            let key = YamlKey::of(&key_value)?;
            (
                key,
                false,
                line.text[key_offset + rest_start..]
                    .trim_start()
                    .to_string(),
            )
        } else {
            let (key_raw, rest_start) = find_key_split(after_props)?;
            let rest_start = key_offset + rest_start;
            let key_value = read_key_scalar(&key_raw, &key_props.tag)?;
            let merge_key = key_props.tag == KeyTag::Merge
                || (matches!(key_props.tag, KeyTag::None | KeyTag::NonSpecific) && key_raw == "<<");
            if let Some(name) = &key_props.anchor {
                // The anchor standing in front of a key binds
                // the key scalar, not the mapping the pair
                // stands in.
                bind_key_anchor(state, name, &key_value, merge_key)?;
            }
            let key = YamlKey::of(&key_value)?;
            (
                key,
                merge_key,
                line.text[rest_start..].trim_start().to_string(),
            )
        };
        cursor.pos = li + 1;
        let value = if rest.is_empty() || rest.starts_with('#') {
            following_block_value(cursor, indent, depth, state, &Tag::None)?
        } else {
            match split_properties(&rest, &state.bindings) {
                PropSplit::Refuse => return None,
                PropSplit::Props(props, offset)
                    if rest[offset..].is_empty() || rest[offset..].starts_with('#') =>
                {
                    // Properties standing alone at the end of
                    // the key line bind the value standing on
                    // the following lines.
                    if let Some(name) = &props.anchor {
                        state.mark(name)?;
                    }
                    let built = following_block_value(cursor, indent, depth, state, &props.tag)?;
                    finish_props(state, props, built)?
                }
                _ => parse_inline_value(&rest, cursor, indent, indent, depth + 1, state)?,
            }
        };
        if merge_key {
            merge_sources.push(value);
        } else {
            explicit.push((key, value));
        }
    }
    flatten_mapping(state, merge_sources, explicit)
}

fn parse_sequence(
    cursor: &mut Cursor,
    indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<Built> {
    let mut items: Vec<Built> = Vec::new();
    loop {
        let Some(li) = cursor.peek_content() else {
            break;
        };
        let line = cursor.lines[li].clone();
        if line.indent > indent {
            return None;
        }
        if line.indent < indent || !is_dash_line(&line) {
            break;
        }
        if line.text.starts_with('\t') {
            return None;
        }
        let after = &line.text[1..];
        let content = after.trim_start().to_string();
        if content.is_empty() || content.starts_with('#') {
            cursor.pos = li + 1;
            let value = match cursor.peek_content() {
                None => Built::scalar(YamlValue::Null),
                Some(nj) => {
                    let next = cursor.lines[nj].clone();
                    if next.indent > indent {
                        parse_block(cursor, indent, depth + 1, state)?
                    } else {
                        Built::scalar(YamlValue::Null)
                    }
                }
            };
            items.push(value);
            continue;
        }
        // Properties standing alone on the dash line bind the
        // whole entry the block standing on the following
        // lines reads as, or the empty scalar when no block
        // stands behind them.
        match split_properties(&content, &state.bindings) {
            PropSplit::Refuse => return None,
            PropSplit::Props(props, offset)
                if content[offset..].is_empty() || content[offset..].starts_with('#') =>
            {
                cursor.pos = li + 1;
                if let Some(name) = &props.anchor {
                    state.mark(name)?;
                }
                let built = match cursor.peek_content() {
                    None => Built::empty_scalar(),
                    Some(nj) => {
                        let next = cursor.lines[nj].clone();
                        if next.indent > indent {
                            parse_block_tagged(cursor, indent, depth + 1, state, &props.tag)?
                        } else {
                            Built::empty_scalar()
                        }
                    }
                };
                items.push(finish_props(state, props, built)?);
                continue;
            }
            _ => {}
        }
        let lead = after.len() - after.trim_start().len();
        let virtual_indent = indent + 1 + lead;
        if is_dash_text(&content) || mapping_shape(&content, &state.bindings) {
            let standing_break = line.has_break;
            let standing_kind = line.kind;
            cursor.lines[li] = Line {
                indent: virtual_indent,
                text: content.clone(),
                raw: content,
                has_break: standing_break,
                kind: standing_kind,
            };
            cursor.pos = li;
            let value = parse_block(cursor, indent, depth + 1, state)?;
            items.push(value);
        } else {
            cursor.pos = li + 1;
            let value = parse_inline_value(&content, cursor, indent, indent, depth + 1, state)?;
            items.push(value);
        }
    }
    Some(Built::list(items))
}

/// Find where a mapping key ends on a line, as the byte index
/// just past its colon, together with the raw key text. A line
/// with no key standing on it hands back nothing.
fn find_key_split(text: &str) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let first = *bytes.first()?;
    if first == b'?' && bytes.get(1) == Some(&b':') {
        // The question mark standing against the colon is
        // scalar content and reads as the key, the mark
        // opening an explicit pair only in front of a space.
        let after = 2;
        if after >= bytes.len() || bytes[after] == b' ' || bytes[after] == b'\t' {
            return Some(("?".to_string(), after));
        }
        return None;
    }
    if first == b'"' || first == b'\'' {
        let (_, end) = if first == b'"' {
            parse_double_quoted(text)?
        } else {
            parse_single_quoted(text)?
        };
        let mut index = end;
        while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
            index += 1;
        }
        if bytes.get(index) != Some(&b':') {
            return None;
        }
        let after = index + 1;
        if after < bytes.len() && bytes[after] != b' ' && bytes[after] != b'\t' {
            return None;
        }
        return Some((text[..end].to_string(), after));
    }
    if first == b'[' || first == b'{' {
        return None;
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (position, (index, ch)) in chars.iter().enumerate() {
        if *ch == '#' && position > 0 && matches!(chars[position - 1].1, ' ' | '\t') {
            return None;
        }
        if *ch == ':' {
            let next = chars.get(position + 1);
            if next.is_none() || matches!(next.unwrap().1, ' ' | '\t') {
                let key_raw = text[..*index].trim_end().to_string();
                if !plain_key_stands(&key_raw) {
                    return None;
                }
                return Some((key_raw, index + 1));
            }
        }
    }
    None
}

fn plain_key_stands(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let first = key.chars().next().expect("key");
    if "&*!|>'\"%@`#,[]{}-".contains(first) && (first != '-' || key.len() > 1) {
        // A leading indicator refuses the key, a dash standing
        // alone or opening a word stands.
        if first != '-' || key.starts_with("- ") {
            return false;
        }
    }
    if key.starts_with("? ") || key.starts_with(": ") || key == "?" || key == ":" {
        return false;
    }
    if key.contains(" #") || key.contains("\t#") {
        return false;
    }
    true
}

/// Read a key scalar under the tag class standing in front
/// of it, a conversion tag converting the scalar's own text
/// as it converts a value's text under PORT4-6 and every
/// other standing class reading the scalar as it reads
/// without one.
fn read_key_scalar(raw: &str, tag: &KeyTag) -> Option<YamlValue> {
    match tag {
        KeyTag::Convert(tag) => {
            let text = if let Some(stripped) = raw.strip_prefix('\"') {
                let _ = stripped;
                parse_double_quoted(raw)?.0
            } else if raw.starts_with('\'') {
                parse_single_quoted(raw)?.0
            } else {
                raw.trim().to_string()
            };
            convert_tagged_scalar(tag, &text)
        }
        KeyTag::Refuse => None,
        _ => parse_key_scalar(raw),
    }
}

/// Bind an anchor name to a key value in the anchor table,
/// a name bound anywhere in the document or still building
/// refusing a second binding. The record carries whether
/// the key stood as a merge key, the standing PORT13-3
/// tells apart at an alias standing as a key.
fn bind_key_anchor(
    state: &mut ReadState,
    name: &str,
    value: &YamlValue,
    merge_key: bool,
) -> Option<()> {
    if state.bound.contains_key(name) || state.building.contains(name) {
        return None;
    }
    state.bound.insert(
        name.to_string(),
        AnchorVal {
            value: value.clone(),
            tally: 0,
            items: None,
            merge_key,
        },
    );
    Some(())
}

/// Read an alias standing where a key stands, the resolved
/// node standing as the key where it is a scalar, and hand
/// back the anchor record's merge standing beside the key,
/// so the pair behind the key sorts as a merge where the
/// anchored scalar stood as a merge key, by the plain
/// spelling or under the merge tag, and as an ordinary pair
/// in every other standing, the quoted and string tagged
/// spellings of the merge text among them.
fn read_alias_key(state: &mut ReadState, name: &str) -> Option<(YamlKey, bool)> {
    let merge_key = state.bound.get(name).is_some_and(|entry| entry.merge_key);
    let built = state.resolve_alias(name)?;
    Some((YamlKey::of(&built.value)?, merge_key))
}

fn parse_key_scalar(raw: &str) -> Option<YamlValue> {
    if let Some(stripped) = raw.strip_prefix('"') {
        let _ = stripped;
        let (value, end) = parse_double_quoted(raw)?;
        if end != raw.len() {
            return None;
        }
        return Some(YamlValue::Text(value));
    }
    if raw.starts_with('\'') {
        let (value, end) = parse_single_quoted(raw)?;
        if end != raw.len() {
            return None;
        }
        return Some(YamlValue::Text(value));
    }
    resolve_plain_scalar(raw.trim())
}

/// One unit of a quoted scalar's raw span, as the folding of
/// the seventh slice reads it. A character an escape spells
/// stands as one unit of its own, so the folding never folds
/// it and the trimming never trims it, and a raw break and a
/// raw blank stand as units of theirs, so the folding runs on
/// the span as written and not on the decoded text.
#[derive(Clone, Copy)]
enum SpanUnit {
    /// A letter of the span as written.
    Char(char),
    /// A character an escape spells, standing as content.
    Escaped(char),
    /// A raw break standing in the span, carrying the kind
    /// of the break as it stands.
    Break(BreakKind),
    /// A raw space or tab standing in the span, food for the
    /// trimming that stands beside a break.
    Blank(char),
}

/// The outcome of scanning one quoted span. A span the text
/// ends inside of stands unterminated, and a block reading
/// gathers the lines behind it and scans again, where a span
/// the reading refuses stands refused at once.
enum SpanRead {
    /// The folded text and the byte index just past the
    /// closing quote.
    Closed(String, usize),
    /// The text ends before the closing quote stands.
    Unterminated,
    /// The span stands refused on its own ground.
    Refused,
}

/// Whether a document marker opens at the byte a crossed
/// line's head stands at, the marker spelling followed by a
/// blank or a break or the end of the text.
fn marker_at_head(text: &str, pos: usize) -> bool {
    let rest = &text[pos..];
    if !rest.starts_with("---") && !rest.starts_with("...") {
        return false;
    }
    matches!(
        rest[3..].chars().next(),
        None | Some(' ' | '\t' | '\n' | '\u{2028}' | '\u{2029}')
    )
}

/// Scan the quoted span opening at the head of the text, the
/// raw span read whole. A raw break folds as a unit of the
/// span, a backslash at the end of a line joins the span with
/// nothing standing and takes the blanks that open the next
/// line with it, and a line a span crosses that opens with a
/// document marker refuses the span, the marker standing on
/// the opening line or indented on a crossed line standing as
/// content. The text the reading hands over carries the
/// carriage return forms and the next line character folded
/// to the one line feed from the door, and every other break
/// standing as itself, so the folding states its algebra
/// over the kinds of the breaks.
fn read_quoted_span(text: &str, double: bool) -> SpanRead {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut units: Vec<SpanUnit> = Vec::new();
    let mut index = 1;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        match ch {
            '"' if double => {
                return SpanRead::Closed(fold_span_units(&units), byte + 1);
            }
            '\'' if !double => {
                if chars.get(index + 1).map(|pair| pair.1) == Some('\'') {
                    units.push(SpanUnit::Char('\''));
                    index += 2;
                    continue;
                }
                return SpanRead::Closed(fold_span_units(&units), byte + 1);
            }
            '\\' if double => {
                let Some(&(escape_byte, escape)) = chars.get(index + 1) else {
                    return SpanRead::Unterminated;
                };
                if matches!(escape, '\n' | '\u{2028}' | '\u{2029}') {
                    // A broken line inside a double quoted
                    // scalar joins with nothing standing, and
                    // the marker standing at the head of the
                    // line it crosses refuses as it refuses
                    // across a folded break.
                    if marker_at_head(text, escape_byte + escape.len_utf8()) {
                        return SpanRead::Refused;
                    }
                    index += 2;
                    while matches!(chars.get(index), Some((_, ' ' | '\t'))) {
                        index += 1;
                    }
                    continue;
                }
                index += 2;
                let decoded = match escape {
                    '0' => '\0',
                    'a' => '\u{7}',
                    'b' => '\u{8}',
                    't' | '\t' => '\t',
                    'n' => '\n',
                    'v' => '\u{B}',
                    'f' => '\u{C}',
                    'r' => '\r',
                    'e' => '\u{1B}',
                    '"' => '"',
                    '/' => '/',
                    '\\' => '\\',
                    'N' => '\u{85}',
                    '_' => '\u{A0}',
                    'L' => '\u{2028}',
                    'P' => '\u{2029}',
                    'x' | 'u' | 'U' => {
                        let count = match escape {
                            'x' => 2,
                            'u' => 4,
                            _ => 8,
                        };
                        let mut value: u32 = 0;
                        for _ in 0..count {
                            let Some(&(_, digit)) = chars.get(index) else {
                                return SpanRead::Unterminated;
                            };
                            let Some(digit) = digit.to_digit(16) else {
                                return SpanRead::Refused;
                            };
                            value = value * 16 + digit;
                            index += 1;
                        }
                        match char::from_u32(value) {
                            Some(ch) => ch,
                            None => return SpanRead::Refused,
                        }
                    }
                    _ => return SpanRead::Refused,
                };
                units.push(SpanUnit::Escaped(decoded));
            }
            '\n' | '\u{2028}' | '\u{2029}' => {
                if marker_at_head(text, byte + ch.len_utf8()) {
                    return SpanRead::Refused;
                }
                units.push(SpanUnit::Break(match ch {
                    '\n' => BreakKind::LineFeed,
                    '\u{2028}' => BreakKind::LineSeparator,
                    _ => BreakKind::ParagraphSeparator,
                }));
                index += 1;
            }
            ' ' | '\t' => {
                units.push(SpanUnit::Blank(ch));
                index += 1;
            }
            ch => {
                if (ch as u32) < 0x20 {
                    return SpanRead::Refused;
                }
                units.push(SpanUnit::Char(ch));
                index += 1;
            }
        }
    }
    SpanRead::Unterminated
}

/// Fold the units of one quoted span. The units gather into
/// text pieces and break runs, a blank beside a break
/// standing inside the run and a blank at the edge of a text
/// piece trimming where a run stands beside it, and every
/// run folds by the run rule, one break of line feed kind
/// to one space and one separator standing as itself and a
/// longer run standing as the rule hands it back, at the
/// edges of the span as between its letters.
fn fold_span_units(units: &[SpanUnit]) -> String {
    enum Piece {
        Text(Vec<SpanUnit>),
        Breaks(Vec<BreakKind>),
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for unit in units {
        match unit {
            SpanUnit::Break(kind) => match pieces.last_mut() {
                Some(Piece::Breaks(kinds)) => kinds.push(*kind),
                _ => pieces.push(Piece::Breaks(vec![*kind])),
            },
            SpanUnit::Blank(_) => match pieces.last_mut() {
                Some(Piece::Breaks(_)) => {}
                Some(Piece::Text(text)) => text.push(*unit),
                None => pieces.push(Piece::Text(vec![*unit])),
            },
            SpanUnit::Char(_) | SpanUnit::Escaped(_) => match pieces.last_mut() {
                Some(Piece::Text(text)) => text.push(*unit),
                _ => pieces.push(Piece::Text(vec![*unit])),
            },
        }
    }
    let mut out = String::new();
    for (position, piece) in pieces.iter().enumerate() {
        match piece {
            Piece::Breaks(kinds) => {
                out.push_str(&fold_break_run(kinds));
            }
            Piece::Text(text) => {
                let mut slice: &[SpanUnit] = text;
                if position > 0 {
                    while matches!(slice.first(), Some(SpanUnit::Blank(_))) {
                        slice = &slice[1..];
                    }
                }
                if position + 1 < pieces.len() {
                    while matches!(slice.last(), Some(SpanUnit::Blank(_))) {
                        slice = &slice[..slice.len() - 1];
                    }
                }
                for unit in slice {
                    match unit {
                        SpanUnit::Char(ch) | SpanUnit::Escaped(ch) | SpanUnit::Blank(ch) => {
                            out.push(*ch)
                        }
                        SpanUnit::Break(_) => unreachable!(),
                    }
                }
            }
        }
    }
    out
}

/// Read a quoted scalar standing in a block position, on the
/// line the caller hands over or spanning the lines behind
/// it, handing back the folded text and the rest of the line
/// the closing quote stands on. The lines the span crosses
/// stand gathered as the flow reading gathers them, each
/// line's indent standing in front of its text and the break
/// that ended the line before it standing between them as
/// the character of its kind.
fn read_block_quoted(text: &str, cursor: &mut Cursor, double: bool) -> Option<(String, String)> {
    match read_quoted_span(text, double) {
        SpanRead::Closed(value, end) => {
            return Some((value, text[end..].to_string()));
        }
        SpanRead::Refused => return None,
        SpanRead::Unterminated => {}
    }
    let mut combined = text.to_string();
    let mut pending_kind = if cursor.pos > 0 {
        cursor.lines[cursor.pos - 1].kind
    } else {
        BreakKind::LineFeed
    };
    loop {
        if cursor.pos >= cursor.lines.len() {
            return None;
        }
        let line = cursor.lines[cursor.pos].clone();
        cursor.pos += 1;
        combined.push(pending_kind.as_char());
        combined.push_str(&" ".repeat(line.indent));
        combined.push_str(&line.text);
        pending_kind = line.kind;
        match read_quoted_span(&combined, double) {
            SpanRead::Closed(value, end) => {
                return Some((value, combined[end..].to_string()));
            }
            SpanRead::Refused => return None,
            SpanRead::Unterminated => {}
        }
    }
}

/// Parse a single quoted scalar standing on one line, handing
/// back its text and the byte index just past its close.
fn parse_single_quoted(text: &str) -> Option<(String, usize)> {
    match read_quoted_span(text, false) {
        SpanRead::Closed(value, end) => Some((value, end)),
        _ => None,
    }
}

/// Parse a double quoted scalar standing on one line, handing
/// back its text and the byte index just past its close.
fn parse_double_quoted(text: &str) -> Option<(String, usize)> {
    match read_quoted_span(text, true) {
        SpanRead::Closed(value, end) => Some((value, end)),
        _ => None,
    }
}

fn check_trailing(text: &str) -> Option<()> {
    let rest = text.trim();
    if rest.is_empty() || rest.starts_with('#') {
        Some(())
    } else {
        None
    }
}

/// Cut a plain scalar segment at the comment that ends it.
fn cut_comment(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#'
            && index > 0
            && (bytes[index - 1] == b' ' || bytes[index - 1] == b'\t')
        {
            return &text[..index];
        }
        index += 1;
    }
    text
}

fn plain_segment_stands(segment: &str) -> bool {
    let trimmed = segment.trim();
    !trimmed.contains(": ") && !trimmed.ends_with(':')
}

fn parse_inline_value(
    text: &str,
    cursor: &mut Cursor,
    block_indent: usize,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
) -> Option<Built> {
    if matches!(text.as_bytes().first(), Some(b'&') | Some(b'!')) {
        match split_properties(text, &state.bindings) {
            PropSplit::Refuse => return None,
            PropSplit::Props(props, offset) => {
                let rest = &text[offset..];
                if rest.starts_with('*') {
                    // A property standing in front of an alias
                    // refuses, as the runtime reading refuses
                    // it in its own parser.
                    return None;
                }
                if let Some(name) = &props.anchor {
                    state.mark(name)?;
                }
                let built = if rest.is_empty() || rest.starts_with('#') {
                    read_following_block_inline(
                        cursor,
                        block_indent,
                        parent_indent,
                        depth,
                        state,
                        &props.tag,
                    )?
                } else {
                    parse_inline_node(
                        rest,
                        cursor,
                        block_indent,
                        parent_indent,
                        depth,
                        state,
                        &props.tag,
                    )?
                };
                return finish_props(state, props, built);
            }
            PropSplit::None => {}
        }
    }
    parse_inline_node(
        text,
        cursor,
        block_indent,
        parent_indent,
        depth,
        state,
        &Tag::None,
    )
}

/// Read the node standing on the lines behind a value text
/// that carried properties alone, reached from the scalar
/// dispatch of a block, a dash standing at the value's own
/// indent opening its sequence and a block standing deeper
/// than the parent opening under it.
fn read_following_block_inline(
    cursor: &mut Cursor,
    block_indent: usize,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
    tag: &Tag,
) -> Option<Built> {
    let Some(nj) = cursor.peek_content() else {
        return Some(Built::empty_scalar());
    };
    let next = cursor.lines[nj].clone();
    if next.indent == block_indent && is_dash_line(&next) {
        parse_sequence(cursor, block_indent, depth + 1, state)
    } else if next.indent > parent_indent {
        parse_block_tagged(cursor, parent_indent, depth + 1, state, tag)
    } else {
        Some(Built::empty_scalar())
    }
}

fn parse_inline_node(
    text: &str,
    cursor: &mut Cursor,
    block_indent: usize,
    parent_indent: usize,
    depth: usize,
    state: &mut ReadState,
    tag: &Tag,
) -> Option<Built> {
    let first = text.chars().next()?;
    if first == '|' || first == '>' {
        let value = parse_block_scalar(text, cursor, parent_indent, &state.bindings)?;
        let YamlValue::Text(assembled) = value else {
            return None;
        };
        if tag.is_constructor() {
            // No trimming stands between the assembly and the
            // conversion.
            let converted = convert_tagged_scalar(tag, &assembled)?;
            return Some(Built::converted(converted));
        }
        return Some(Built::from_text(assembled));
    }
    match first {
        '[' | '{' => {
            let mut combined = text.to_string();
            let mut pending_kind = if cursor.pos > 0 {
                cursor.lines[cursor.pos - 1].kind
            } else {
                BreakKind::LineFeed
            };
            while !flow_closed(&combined) {
                if cursor.pos >= cursor.lines.len() {
                    return None;
                }
                let line = cursor.lines[cursor.pos].clone();
                cursor.pos += 1;
                combined.push(pending_kind.as_char());
                combined.push_str(&" ".repeat(line.indent));
                combined.push_str(&line.text);
                pending_kind = line.kind;
            }
            let mut parser = FlowParser::new(&combined, state);
            let value = parser.parse_value(depth)?;
            parser.skip_separators();
            if parser.pos != parser.chars.len() {
                return None;
            }
            Some(value)
        }
        '"' => {
            let (value, rest) = read_block_quoted(text, cursor, true)?;
            check_trailing(&rest)?;
            if tag.is_constructor() {
                let converted = convert_tagged_scalar(tag, &value)?;
                return Some(Built::converted(converted));
            }
            Some(Built::from_text(value))
        }
        '\'' => {
            let (value, rest) = read_block_quoted(text, cursor, false)?;
            check_trailing(&rest)?;
            if tag.is_constructor() {
                let converted = convert_tagged_scalar(tag, &value)?;
                return Some(Built::converted(converted));
            }
            Some(Built::from_text(value))
        }
        '*' => {
            // An alias reads as a copy of the anchored value
            // standing under its name, the whole text standing
            // as the alias token and a comment at most.
            let bytes = text.as_bytes();
            let mut end = 1;
            while end < bytes.len() && is_anchor_char(bytes[end]) {
                end += 1;
            }
            if end == 1 {
                return None;
            }
            check_trailing(&text[end..])?;
            state.resolve_alias(&text[1..end])
        }
        '&' | '!' | '%' | '@' | '`' | ']' | '}' | ',' => None,
        '?' if text == "?" || text.starts_with("? ") => None,
        ':' if text == ":" || text.starts_with(": ") => None,
        '-' if text == "-" || text.starts_with("- ") => None,
        '#' => Some(Built::scalar(YamlValue::Null)),
        _ => {
            let first_cut = cut_comment(text);
            if !plain_segment_stands(first_cut) {
                return None;
            }
            // A tab inside a plain scalar refuses the reading,
            // as the runtime scanner refuses it, whatever a
            // stray quote earlier on the line suggested to the
            // tab scan of the whole text.
            if first_cut.contains('\t') {
                return None;
            }
            let mut chunks: Vec<String> = vec![first_cut.trim().to_string()];
            let mut runs: Vec<Vec<BreakKind>> = Vec::new();
            let mut pending: Vec<BreakKind> = Vec::new();
            let mut prev_line = cursor.pos.saturating_sub(1);
            let mut scan = cursor.pos;
            let mut comment_seen = false;
            loop {
                let Some(line) = cursor.lines.get(scan) else {
                    break;
                };
                let line = line.clone();
                let trimmed = line.text.trim();
                if trimmed.is_empty() {
                    pending.push(line.kind);
                    scan += 1;
                    continue;
                }
                if trimmed.starts_with('#') {
                    comment_seen = true;
                    scan += 1;
                    continue;
                }
                if line.indent <= block_indent {
                    break;
                }
                // A plain scalar does not fold across a comment
                // line, as the runtime reading refuses a scalar
                // whose continuation stands behind one.
                if comment_seen {
                    return None;
                }
                let segment = cut_comment(&line.text);
                if !plain_segment_stands(segment) {
                    return None;
                }
                if segment.contains('\t') {
                    return None;
                }
                let mut run = vec![cursor.lines[prev_line].kind];
                run.extend(pending.iter().copied());
                runs.push(run);
                pending.clear();
                chunks.push(segment.trim().to_string());
                prev_line = scan;
                scan += 1;
                cursor.pos = scan;
            }
            let mut folded = chunks[0].clone();
            for (run, chunk) in runs.iter().zip(chunks.iter().skip(1)) {
                folded.push_str(&fold_break_run(run));
                folded.push_str(chunk);
            }
            if tag.is_constructor() {
                let converted = convert_tagged_scalar(tag, &folded)?;
                return Some(Built::converted(converted));
            }
            if folded == "<<" {
                // The plain merge spelling standing in value
                // position refuses, its standing reading as
                // plain text in value position ended with
                // this slice.
                return None;
            }
            let value = resolve_plain_scalar(&folded)?;
            Some(Built {
                value,
                tally: 0,
                items: None,
                text: Some(folded),
                plain_token: None,
                tag_done: false,
            })
        }
    }
}

// ----------------------------------------------------------------
// Block scalars, PORT3
// ----------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Chomping {
    Clip,
    Strip,
    Keep,
}

/// The shape a dedented first line takes when it is no
/// sibling and no content, the shapes under which PORT3-2
/// reads the scalar as empty and lets the document stand or
/// refuse on the line as it stands outside a block. A line
/// parses as a sibling when it stands at or above the holding
/// node's indent in a mapping entry or a dash entry shape,
/// and a document end marker at column 0 ends the scalar the
/// same way.
fn sibling_or_end_shape(
    line: &Line,
    parent_indent: usize,
    bindings: &HashMap<String, String>,
) -> bool {
    if line.indent == 0 && (line.text == "..." || line.text.starts_with("... ")) {
        return true;
    }
    line.indent <= parent_indent && (mapping_shape(&line.text, bindings) || is_dash_line(line))
}

/// Read a block scalar whose head stands in `text`, the lines
/// of the scalar standing in the cursor at its standing
/// position, under PORT3-1 to PORT3-5. The holding node's
/// indent is `parent_indent`. On a reading the cursor stands
/// on the first line that is not part of the scalar.
fn parse_block_scalar(
    text: &str,
    cursor: &mut Cursor,
    parent_indent: usize,
    bindings: &HashMap<String, String>,
) -> Option<YamlValue> {
    let bytes = text.as_bytes();
    let folded = bytes[0] == b'>';
    let mut index = 1usize;
    let mut chomping = Chomping::Clip;
    let mut chomping_seen = false;
    let mut digit: Option<usize> = None;
    while index < bytes.len() {
        match bytes[index] {
            b'+' if !chomping_seen => {
                chomping = Chomping::Keep;
                chomping_seen = true;
                index += 1;
            }
            b'-' if !chomping_seen => {
                chomping = Chomping::Strip;
                chomping_seen = true;
                index += 1;
            }
            b'1'..=b'9' if digit.is_none() => {
                digit = Some((bytes[index] - b'0') as usize);
                index += 1;
            }
            _ => break,
        }
    }
    // The header line after the indicators holds only spaces
    // and then optionally a comment, and a comment stands
    // only where a space stands in front of its hash, so a
    // hash standing directly against the mark or an indicator
    // refuses.
    let rest = &text[index..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    check_trailing(rest)?;
    let explicit = digit.map(|digit| parent_indent + digit);
    // Gather the lines of the block, walking the line vector
    // directly, no line skipped as blank or as a comment.
    let mut raw_lines: Vec<(String, bool, BreakKind)> = Vec::new();
    let mut scan = cursor.pos;
    let mut block_indent: Option<usize> = explicit;
    let mut floor = 0usize;
    let mut decided = explicit.is_some();
    let mut first_content: Option<usize> = None;
    while scan < cursor.lines.len() {
        let line = cursor.lines[scan].clone();
        let is_blank = line.text.is_empty();
        if !decided {
            if is_blank {
                if line.indent > floor {
                    floor = line.indent;
                }
                raw_lines.push((line.raw.clone(), line.has_break, line.kind));
                scan += 1;
                continue;
            }
            if line.indent <= parent_indent {
                break;
            }
            if line.indent < floor {
                if sibling_or_end_shape(&line, parent_indent, bindings) {
                    break;
                }
                return None;
            }
            block_indent = Some(line.indent);
            decided = true;
            first_content = Some(raw_lines.len());
            raw_lines.push((line.raw.clone(), line.has_break, line.kind));
            scan += 1;
            continue;
        }
        let width = block_indent.expect("decided width");
        if is_blank {
            raw_lines.push((line.raw.clone(), line.has_break, line.kind));
            scan += 1;
            continue;
        }
        if line.indent < width {
            if first_content.is_none() {
                if sibling_or_end_shape(&line, parent_indent, bindings) {
                    break;
                }
                return None;
            }
            break;
        }
        if first_content.is_none() {
            first_content = Some(raw_lines.len());
        }
        raw_lines.push((line.raw.clone(), line.has_break, line.kind));
        scan += 1;
    }
    let width = block_indent.unwrap_or(parent_indent + 1);
    // Where no content line ever stood, the block indent was
    // never decided, and every gathered line is a blank line
    // standing before a sibling or the end of the document.
    // Such a line contributes its break alone, however many
    // spaces it carries, as PORT3-3 states for a leading
    // spaces only line, so it is classified blank here
    // rather than read against a width nothing settled.
    let undecided = block_indent.is_none();
    cursor.pos = scan;
    // Strip the block indent and classify every gathered line.
    struct Piece {
        content: String,
        blank: bool,
        more: bool,
        has_break: bool,
        kind: BreakKind,
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for (raw, has_break, kind) in &raw_lines {
        let spaces = raw.chars().take_while(|ch| *ch == ' ').count();
        let all_spaces = raw.chars().all(|ch| ch == ' ');
        if undecided || (all_spaces && spaces <= width) {
            pieces.push(Piece {
                content: String::new(),
                blank: true,
                more: false,
                has_break: *has_break,
                kind: *kind,
            });
        } else {
            let content: String = raw.chars().skip(width).collect();
            let more = spaces > width || content.starts_with('\t');
            pieces.push(Piece {
                content,
                blank: false,
                more,
                has_break: *has_break,
                kind: *kind,
            });
        }
    }
    // Assemble the value before its final breaks, gathering
    // the breaks that stand at its end for the chomping.
    // Every piece carries the kind of the break that ended
    // its line, and the assembly states its joins over the
    // runs of kinds standing between content pieces, a
    // literal join standing as every break of its run and a
    // folded join standing as the run rule hands it back
    // beside the more indented standings.
    let first = pieces.iter().position(|piece| !piece.blank);
    let last = pieces.iter().rposition(|piece| !piece.blank);
    let (body, trailing) = match (first, last) {
        (Some(first), Some(last)) => {
            // Blank lines standing before the content count
            // one break each in the value, under every
            // chomping, once content follows, each standing
            // as the character of its own kind.
            let mut body = String::new();
            for piece in &pieces[..first] {
                body.push(piece.kind.as_char());
            }
            body.push_str(&pieces[first].content);
            let mut index = first + 1;
            while index <= last {
                if pieces[index].blank {
                    index += 1;
                    continue;
                }
                let mut back = index - 1;
                while pieces[back].blank {
                    back -= 1;
                }
                let run: Vec<BreakKind> = (back..index).map(|at| pieces[at].kind).collect();
                if !folded {
                    for kind in &run {
                        body.push(kind.as_char());
                    }
                } else if run.len() == 1 {
                    if run[0].is_separator() {
                        body.push(run[0].as_char());
                    } else if pieces[back].more || pieces[index].more {
                        body.push('\n');
                    } else {
                        body.push(' ');
                    }
                } else if run[0].is_separator() || pieces[back].more || pieces[index].more {
                    for kind in &run {
                        body.push(kind.as_char());
                    }
                } else {
                    for kind in &run[1..] {
                        body.push(kind.as_char());
                    }
                }
                body.push_str(&pieces[index].content);
                index += 1;
            }
            let trailing: Vec<BreakKind> = (last..pieces.len())
                .filter(|at| pieces[*at].has_break)
                .map(|at| pieces[at].kind)
                .collect();
            (body, trailing)
        }
        _ => {
            let trailing: Vec<BreakKind> = pieces
                .iter()
                .filter(|piece| piece.has_break)
                .map(|piece| piece.kind)
                .collect();
            (String::new(), trailing)
        }
    };
    let value = match chomping {
        Chomping::Strip => body,
        Chomping::Clip => {
            if body.is_empty() || trailing.is_empty() {
                body
            } else {
                format!("{body}{}", trailing[0].as_char())
            }
        }
        Chomping::Keep => {
            let mut kept = body;
            for kind in &trailing {
                kept.push(kind.as_char());
            }
            kept
        }
    };
    Some(YamlValue::Text(value))
}

/// Whether the brackets of a flow text stand balanced, quotes
/// taken into account. A quote opens a span only where a
/// quoted scalar can stand, under PORT11-3: where the entry it
/// stands in carries no plain content before it. A quote
/// standing behind the plain content of its entry is content
/// of that entry's plain scalar and opens nothing. An entry
/// begins at the text's start and behind every flow bracket
/// and comma and behind the colon that divides a key from its
/// value, the colon dividing only where the plain token scan
/// lets it divide. An entry's properties, an anchor or an
/// alias or a tag, stand before its scalar and do not begin
/// it, so a quoted scalar behind a property still opens.
/// A hash opens a comment only where no plain content
/// stands directly before it in its entry, under PORT12-3.
/// A hash standing directly behind plain content is content
/// of the entry's plain scalar and opens nothing. Plain
/// content stands directly before a hash where the character
/// before the hash is a character of the entry's plain
/// scalar, and a blank or a flow bracket or a comma or a
/// dividing colon or the close of a quoted span or of a
/// nested collection breaks the standing. A separator
/// stands at the end of a line in this reading as a line
/// feed stands, the gathering having handed the lines over
/// with the breaks of their kinds standing between them.
fn flow_closed(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut depth = 0i64;
    let mut index = 0usize;
    let mut quote: Option<char> = None;
    let mut scalar_begun = false;
    let mut plain_attached = false;
    while index < chars.len() {
        let ch = chars[index];
        match quote {
            Some('"') => match ch {
                '\\' => {
                    index += 1;
                }
                '"' => {
                    quote = None;
                    scalar_begun = true;
                    plain_attached = false;
                }
                _ => {}
            },
            Some('\'') => {
                if ch == '\'' {
                    if chars.get(index + 1) == Some(&'\'') {
                        index += 1;
                    } else {
                        quote = None;
                        scalar_begun = true;
                        plain_attached = false;
                    }
                }
            }
            _ => match ch {
                '"' | '\'' => {
                    if !scalar_begun {
                        quote = Some(ch);
                        plain_attached = false;
                    } else {
                        // A quote standing behind the plain
                        // content of its entry is a character
                        // of that entry's plain scalar, so the
                        // content attaches through it.
                        plain_attached = true;
                    }
                }
                '[' | '{' => {
                    depth += 1;
                    scalar_begun = false;
                    plain_attached = false;
                }
                ']' | '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return true;
                    }
                    scalar_begun = true;
                    plain_attached = false;
                }
                ',' => {
                    scalar_begun = false;
                    plain_attached = false;
                }
                ':' => {
                    let next = chars.get(index + 1);
                    scalar_begun = !(next.is_none()
                        || matches!(
                            next,
                            Some(
                                ' ' | '\t'
                                    | '\n'
                                    | '\u{2028}'
                                    | '\u{2029}'
                                    | ','
                                    | ']'
                                    | '}'
                                    | '['
                                    | '{'
                            )
                        ));
                    // The colon carries the standing only
                    // where plain content stood directly
                    // before it and the colon does not
                    // divide, under the amendment to
                    // PORT12-3. A colon at an entry's start
                    // or behind a closed collection is no
                    // character of a plain scalar.
                    plain_attached = plain_attached && scalar_begun;
                }
                '?' if !scalar_begun => {
                    // The explicit key mark at an entry's
                    // start, standing before its scalar and
                    // not beginning it, under the amendment
                    // to PORT12-3. The mark is no character
                    // of a plain scalar, so a hash standing
                    // directly behind it opens a comment.
                    plain_attached = false;
                }
                '#' => {
                    if !plain_attached {
                        // A comment runs to the end of its line.
                        while index + 1 < chars.len()
                            && !matches!(chars[index + 1], '\n' | '\u{2028}' | '\u{2029}')
                        {
                            index += 1;
                        }
                    }
                    // A hash standing directly behind the
                    // plain content of its entry is content
                    // of that entry's plain scalar, under
                    // PORT12-3, and opens nothing.
                }
                '&' | '*' | '!' if !scalar_begun => {
                    // A property of the entry, standing before
                    // its scalar and not beginning it. The
                    // property's token runs to its separator.
                    while index + 1 < chars.len()
                        && !matches!(
                            chars[index + 1],
                            ' ' | '\t'
                                | '\n'
                                | '\u{2028}'
                                | '\u{2029}'
                                | ','
                                | '['
                                | ']'
                                | '{'
                                | '}'
                        )
                    {
                        index += 1;
                    }
                }
                ' ' | '\t' | '\n' | '\u{2028}' | '\u{2029}' => {
                    plain_attached = false;
                }
                _ => {
                    scalar_begun = true;
                    plain_attached = true;
                }
            },
        }
        index += 1;
    }
    depth == 0 && quote.is_none()
}

// ----------------------------------------------------------------
// Flow collections
// ----------------------------------------------------------------

struct FlowParser<'a> {
    chars: Vec<char>,
    pos: usize,
    state: &'a mut ReadState,
    // The standing of the node the last parse_value read at
    // its own level, its plain token when it stood as one and
    // the properties that stood in front of it, for the
    // single pair and merge readings of its consumers.
    last_plain: Option<String>,
    last_anchor: bool,
    last_tag: Tag,
}

impl<'a> FlowParser<'a> {
    fn new(text: &str, state: &'a mut ReadState) -> FlowParser<'a> {
        FlowParser {
            chars: text.chars().collect(),
            pos: 0,
            state,
            last_plain: None,
            last_anchor: false,
            last_tag: Tag::None,
        }
    }

    fn skip_separators(&mut self) {
        loop {
            match self.chars.get(self.pos) {
                Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('\u{2028}')
                | Some('\u{2029}') => {
                    self.pos += 1;
                }
                Some('#') => {
                    while let Some(ch) = self.chars.get(self.pos) {
                        self.pos += 1;
                        if matches!(*ch, '\n' | '\u{2028}' | '\u{2029}') {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Take the properties standing in front of a flow node,
    /// at most one anchor and at most one tag in either order.
    fn take_properties(&mut self) -> Option<Props> {
        let mut props = Props::new();
        loop {
            match self.peek()? {
                '&' => {
                    if props.anchor.is_some() {
                        return None;
                    }
                    let start = self.pos + 1;
                    let mut end = start;
                    while end < self.chars.len() && is_anchor_char_char(self.chars[end]) {
                        end += 1;
                    }
                    if end == start {
                        return None;
                    }
                    match self.chars.get(end) {
                        None
                        | Some(
                            ' ' | '\t' | '\n' | '\r' | '\u{2028}' | '\u{2029}' | ',' | ']' | '}'
                            | ':',
                        ) => {}
                        _ => return None,
                    }
                    props.anchor = Some(self.chars[start..end].iter().collect());
                    self.pos = end;
                }
                '!' => {
                    if props.tag != Tag::None {
                        return None;
                    }
                    props.tag = self.take_tag_token()?;
                }
                _ => break,
            }
            self.skip_separators();
            if !matches!(self.peek(), Some('&') | Some('!')) {
                break;
            }
        }
        Some(props)
    }

    /// Take the tag token standing at the cursor, resolving
    /// its spelling under the standing bindings.
    fn take_tag_token(&mut self) -> Option<Tag> {
        if self.chars.get(self.pos + 1) == Some(&'<') {
            let mut end = self.pos + 2;
            while end < self.chars.len() && self.chars[end] != '>' {
                end += 1;
            }
            if end >= self.chars.len() {
                return None;
            }
            let full: String = self.chars[self.pos + 2..end].iter().collect();
            if full.is_empty() {
                return None;
            }
            let after = end + 1;
            match self.chars.get(after) {
                None | Some(' ' | '\t' | '\n' | '\r' | ',' | ']' | '}' | ':') => {}
                _ => return None,
            }
            self.pos = after;
            return Some(classify_tag(&full));
        }
        let mut end = self.pos;
        while end < self.chars.len()
            && !matches!(
                self.chars[end],
                ' ' | '\t' | '\n' | '\r' | '\u{2028}' | '\u{2029}' | ',' | ']' | '}'
            )
        {
            end += 1;
        }
        let token: String = self.chars[self.pos..end].iter().collect();
        self.pos = end;
        resolve_tag_spelling(&token, &self.state.bindings)
    }

    /// Take the one tag token a flow key can carry, under
    /// the classes key position allows.
    fn take_key_tag(&mut self) -> Option<KeyTag> {
        match key_tag_of(&self.take_tag_token()?) {
            KeyTag::Refuse => None,
            tag => Some(tag),
        }
    }

    /// Take the properties standing in front of a flow key,
    /// at most one anchor and at most one tag in either
    /// order, under the classes key position allows.
    fn take_key_properties(&mut self) -> Option<(Option<String>, KeyTag)> {
        let mut anchor: Option<String> = None;
        let mut tag = KeyTag::None;
        loop {
            match self.peek()? {
                '&' => {
                    if anchor.is_some() {
                        return None;
                    }
                    let start = self.pos + 1;
                    let mut end = start;
                    while end < self.chars.len() && is_anchor_char_char(self.chars[end]) {
                        end += 1;
                    }
                    if end == start {
                        return None;
                    }
                    match self.chars.get(end) {
                        None | Some(' ' | '\t' | '\n' | '\r' | ',' | ']' | '}' | ':') => {}
                        _ => return None,
                    }
                    anchor = Some(self.chars[start..end].iter().collect());
                    self.pos = end;
                }
                '!' => {
                    if tag != KeyTag::None {
                        return None;
                    }
                    tag = self.take_key_tag()?;
                }
                _ => break,
            }
            self.skip_separators();
        }
        Some((anchor, tag))
    }

    fn parse_value(&mut self, depth: usize) -> Option<Built> {
        if depth > MAX_YAML_NESTING {
            return None;
        }
        self.skip_separators();
        let mut props = Props::new();
        if matches!(self.peek(), Some('&') | Some('!')) {
            props = self.take_properties()?;
            self.skip_separators();
            if self.peek() == Some('*') {
                // A property standing in front of an alias
                // refuses, as the runtime reading refuses it.
                return None;
            }
            if let Some(name) = &props.anchor {
                self.state.mark(name)?;
            }
            if matches!(self.peek(), Some(',') | Some(']') | Some('}')) {
                // The property stands in front of no node, the
                // empty scalar standing as its node.
                let built = finish_props(self.state, props.clone(), Built::empty_scalar())?;
                self.last_plain = None;
                self.last_anchor = props.anchor.is_some();
                self.last_tag = props.tag.clone();
                return Some(built);
            }
        }
        let tag = props.tag.clone();
        let built = self.parse_flow_node(depth, &tag)?;
        let built = finish_props(self.state, props.clone(), built)?;
        self.last_plain = built.plain_token.clone();
        self.last_anchor = props.anchor.is_some();
        self.last_tag = props.tag.clone();
        Some(built)
    }

    fn parse_flow_node(&mut self, depth: usize, tag: &Tag) -> Option<Built> {
        match self.peek()? {
            '[' => {
                self.pos += 1;
                let mut items: Vec<Built> = Vec::new();
                loop {
                    self.skip_separators();
                    if self.peek() == Some(']') {
                        self.pos += 1;
                        return Some(Built::list(items));
                    }
                    let item = self.parse_value(depth + 1)?;
                    let item_plain = self.last_plain.clone();
                    let item_anchor = self.last_anchor;
                    let item_tag = self.last_tag.clone();
                    self.skip_separators();
                    let item = if self.peek() == Some(':') {
                        // A single pair mapping standing in a
                        // sequence, or a merge pair standing as
                        // one.
                        self.pos += 1;
                        self.skip_separators();
                        let value = match self.peek() {
                            Some(',') | Some(']') => Built::scalar(YamlValue::Null),
                            _ => self.parse_value(depth + 1)?,
                        };
                        if self.last_plain.as_deref() == Some("<<") {
                            return None;
                        }
                        if item_plain.as_deref() == Some("<<") || item_tag == Tag::Merge {
                            let mut merged: Vec<(YamlKey, YamlValue)> = Vec::new();
                            let mut tally = 0usize;
                            expand_merge_into(self.state, value, &mut merged, &mut tally)?;
                            let mut pairs: Vec<(YamlKey, YamlValue)> = Vec::new();
                            for (key, value) in merged {
                                insert_pair(&mut pairs, key, value);
                            }
                            Built {
                                value: YamlValue::Mapping(pairs),
                                tally,
                                items: None,
                                text: None,
                                plain_token: None,
                                tag_done: false,
                            }
                        } else {
                            let _ = item_anchor;
                            if matches!(item_tag, Tag::Seq | Tag::Map | Tag::Refuse) {
                                return None;
                            }
                            let key = YamlKey::of(&item.value)?;
                            Built {
                                value: YamlValue::Mapping(vec![(key, value.value)]),
                                tally: item.tally + value.tally,
                                items: None,
                                text: None,
                                plain_token: None,
                                tag_done: false,
                            }
                        }
                    } else {
                        if item_plain.as_deref() == Some("<<") {
                            // The plain merge spelling standing
                            // in entry position refuses.
                            return None;
                        }
                        item
                    };
                    items.push(item);
                    self.skip_separators();
                    match self.peek() {
                        Some(',') => {
                            self.pos += 1;
                        }
                        Some(']') => {
                            self.pos += 1;
                            return Some(Built::list(items));
                        }
                        _ => return None,
                    }
                }
            }
            '{' => {
                self.pos += 1;
                let mut explicit: Vec<(YamlKey, Built)> = Vec::new();
                let mut merge_sources: Vec<Built> = Vec::new();
                loop {
                    self.skip_separators();
                    if self.peek() == Some('}') {
                        self.pos += 1;
                        break;
                    }
                    // The key stands read by a scalar path of
                    // its own, the properties in front of it
                    // standing under the classes key position
                    // allows, an anchor binding the key node
                    // and a conversion tag converting the key
                    // scalar and an alias standing as the key.
                    let (mut key_anchor, mut key_tag) =
                        if matches!(self.peek(), Some('&') | Some('!')) {
                            self.take_key_properties()?
                        } else {
                            (None, KeyTag::None)
                        };
                    self.skip_separators();
                    let mut alias_key: Option<(YamlKey, bool)> = None;
                    if self.peek() == Some('*') {
                        // An alias standing as the key, a
                        // property in front of an alias
                        // refusing as it refuses in value
                        // position.
                        if key_anchor.is_some() || key_tag != KeyTag::None {
                            return None;
                        }
                        let start = self.pos + 1;
                        let mut end = start;
                        while end < self.chars.len() && is_anchor_char_char(self.chars[end]) {
                            end += 1;
                        }
                        if end == start {
                            return None;
                        }
                        let name: String = self.chars[start..end].iter().collect();
                        self.pos = end;
                        alias_key = Some(read_alias_key(self.state, &name)?);
                        self.skip_separators();
                    }
                    let mut empty_explicit_key = false;
                    let mut explicit_pair = false;
                    if alias_key.is_none()
                        && self.peek() == Some('?')
                        && matches!(
                            self.chars.get(self.pos + 1),
                            Some(' ') | Some('}') | Some(',')
                        )
                    {
                        explicit_pair = true;
                        // An explicit key opens with the
                        // question mark, the key node standing
                        // behind it, and an empty key standing
                        // when the pair or the mapping closes
                        // at once behind the mark. The key's
                        // properties stand behind the marker as
                        // they stand in front of a compact key,
                        // under the same classes.
                        self.pos += 1;
                        self.skip_separators();
                        if matches!(self.peek(), Some('&') | Some('!')) {
                            let (more_anchor, more_tag) = self.take_key_properties()?;
                            if more_anchor.is_some() {
                                if key_anchor.is_some() {
                                    return None;
                                }
                                key_anchor = more_anchor;
                            }
                            if more_tag != KeyTag::None {
                                if key_tag != KeyTag::None {
                                    return None;
                                }
                                key_tag = more_tag;
                            }
                            self.skip_separators();
                        }
                        if self.peek() == Some('*') {
                            if key_anchor.is_some() || key_tag != KeyTag::None {
                                return None;
                            }
                            let start = self.pos + 1;
                            let mut end = start;
                            while end < self.chars.len() && is_anchor_char_char(self.chars[end]) {
                                end += 1;
                            }
                            if end == start {
                                return None;
                            }
                            let name: String = self.chars[start..end].iter().collect();
                            self.pos = end;
                            alias_key = Some(read_alias_key(self.state, &name)?);
                            self.skip_separators();
                        }
                        if matches!(self.peek(), Some('}') | Some(',')) {
                            empty_explicit_key = true;
                        }
                    }
                    let (key_value, key_plain) = if alias_key.is_some() || empty_explicit_key {
                        (YamlValue::Null, None)
                    } else {
                        match self.peek()? {
                            '"' => {
                                let text: String = self.chars[self.pos..].iter().collect();
                                let (value, end) = parse_flow_double_quoted(&text)?;
                                if !explicit_pair
                                    && text
                                        .chars()
                                        .take(end)
                                        .any(|ch| matches!(ch, '\n' | '\u{2028}' | '\u{2029}'))
                                {
                                    // A quoted scalar spanning
                                    // lines standing as a compact
                                    // flow key refuses, as the
                                    // runtime refuses it.
                                    return None;
                                }
                                self.pos += end;
                                match &key_tag {
                                    KeyTag::Convert(tag) => {
                                        (convert_tagged_scalar(tag, &value)?, None)
                                    }
                                    _ => (YamlValue::Text(value), None),
                                }
                            }
                            '\'' => {
                                let text: String = self.chars[self.pos..].iter().collect();
                                let (value, end) = parse_flow_single_quoted(&text)?;
                                if !explicit_pair
                                    && text
                                        .chars()
                                        .take(end)
                                        .any(|ch| matches!(ch, '\n' | '\u{2028}' | '\u{2029}'))
                                {
                                    return None;
                                }
                                self.pos += end;
                                match &key_tag {
                                    KeyTag::Convert(tag) => {
                                        (convert_tagged_scalar(tag, &value)?, None)
                                    }
                                    _ => (YamlValue::Text(value), None),
                                }
                            }
                            '[' | '{' => {
                                let built = self.parse_value(depth + 1)?;
                                match &key_tag {
                                    KeyTag::Convert(tag) => (apply_tag(tag, built)?.value, None),
                                    _ => (built.value, None),
                                }
                            }
                            ',' | '}' | ':' | '|' | '>' | '%' | '@' | '`' => return None,
                            _ => {
                                let token_start = self.pos;
                                let folded = self.scan_plain_token()?;
                                let scanned: String =
                                    self.chars[token_start..self.pos].iter().collect();
                                if !explicit_pair
                                    && (scanned.contains('\u{2028}')
                                        || scanned.contains('\u{2029}'))
                                {
                                    // A plain key carrying a
                                    // separator refuses in flow
                                    // position, under PORT15-5,
                                    // the check reading the
                                    // token as it stands
                                    // scanned, before its
                                    // trailing break run folds
                                    // away, so a separator at
                                    // the token's tail refuses
                                    // as one inside it does,
                                    // and the explicit key form
                                    // standing outside the
                                    // refusal as the runtime
                                    // reads it.
                                    return None;
                                }
                                match &key_tag {
                                    KeyTag::Convert(tag) => {
                                        (convert_tagged_scalar(tag, &folded)?, None)
                                    }
                                    _ => {
                                        let plain = if folded == "<<" {
                                            Some(folded.clone())
                                        } else {
                                            None
                                        };
                                        (resolve_plain_scalar(&folded)?, plain)
                                    }
                                }
                            }
                        }
                    };
                    let merge_key = key_tag == KeyTag::Merge
                        || (matches!(key_tag, KeyTag::None | KeyTag::NonSpecific)
                            && key_plain.as_deref() == Some("<<"));
                    if let Some(name) = &key_anchor {
                        bind_key_anchor(self.state, name, &key_value, merge_key)?;
                    }
                    self.skip_separators();
                    let value = if self.peek() == Some(':') {
                        self.pos += 1;
                        self.skip_separators();
                        match self.peek() {
                            Some(',') | Some('}') => Built::scalar(YamlValue::Null),
                            _ => {
                                let value = self.parse_value(depth + 1)?;
                                if self.last_plain.as_deref() == Some("<<") {
                                    // The plain merge spelling
                                    // standing in value position
                                    // refuses.
                                    return None;
                                }
                                value
                            }
                        }
                    } else {
                        Built::scalar(YamlValue::Null)
                    };
                    if merge_key || alias_key.as_ref().is_some_and(|(_, standing)| *standing) {
                        merge_sources.push(value);
                    } else {
                        let key = match &alias_key {
                            Some((key, _)) => key.clone(),
                            None => YamlKey::of(&key_value)?,
                        };
                        explicit.push((key, value));
                    }
                    self.skip_separators();
                    match self.peek() {
                        Some(',') => {
                            self.pos += 1;
                        }
                        Some('}') => {
                            self.pos += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
                flatten_mapping(self.state, merge_sources, explicit)
            }
            '"' => {
                let text: String = self.chars[self.pos..].iter().collect();
                let (value, end) = parse_flow_double_quoted(&text)?;
                self.pos += end;
                if tag.is_constructor() {
                    let converted = convert_tagged_scalar(tag, &value)?;
                    return Some(Built::converted(converted));
                }
                Some(Built::from_text(value))
            }
            '\'' => {
                let text: String = self.chars[self.pos..].iter().collect();
                let (value, end) = parse_flow_single_quoted(&text)?;
                self.pos += end;
                if tag.is_constructor() {
                    let converted = convert_tagged_scalar(tag, &value)?;
                    return Some(Built::converted(converted));
                }
                Some(Built::from_text(value))
            }
            '*' => {
                let start = self.pos + 1;
                let mut end = start;
                while end < self.chars.len() && is_anchor_char_char(self.chars[end]) {
                    end += 1;
                }
                if end == start {
                    return None;
                }
                let name: String = self.chars[start..end].iter().collect();
                self.pos = end;
                self.state.resolve_alias(&name)
            }
            ',' | ']' | '}' | ':' | '|' | '>' | '%' | '@' | '`' => None,
            _ => self.parse_flow_plain(tag),
        }
    }

    /// Scan one plain token standing at the cursor and hand
    /// back its folded text. A token opening with an
    /// indicator stands in the refused family, a block scalar
    /// head or a reserved indicator or a stray property the
    /// property reading did not take.
    fn scan_plain_token(&mut self) -> Option<String> {
        if matches!(
            self.peek(),
            Some('&' | '*' | '!' | '|' | '>' | '%' | '@' | '`')
        ) {
            return None;
        }
        let start = self.pos;
        let mut index = self.pos;
        loop {
            match self.chars.get(index) {
                None => break,
                Some(',') | Some('[') | Some(']') | Some('{') | Some('}') => break,
                Some(':') => {
                    let next = self.chars.get(index + 1);
                    if next.is_none()
                        || matches!(
                            next,
                            Some(
                                ' ' | '\t'
                                    | '\n'
                                    | '\u{2028}'
                                    | '\u{2029}'
                                    | ','
                                    | ']'
                                    | '}'
                                    | '['
                                    | '{'
                            )
                        )
                    {
                        break;
                    }
                    index += 1;
                }
                Some('#') => {
                    if index > start
                        && matches!(
                            self.chars[index - 1],
                            ' ' | '\t' | '\n' | '\u{2028}' | '\u{2029}'
                        )
                    {
                        break;
                    }
                    index += 1;
                }
                Some(_) => index += 1,
            }
        }
        let raw: String = self.chars[start..index].iter().collect();
        self.pos = index;
        // A tab inside a flow plain scalar refuses the
        // reading, as it does in block position.
        if raw.contains('\t') {
            return None;
        }
        let folded = fold_flow_plain(&raw);
        if folded.is_empty() {
            return None;
        }
        Some(folded)
    }

    fn parse_flow_plain(&mut self, tag: &Tag) -> Option<Built> {
        let folded = self.scan_plain_token()?;
        if tag.is_constructor() {
            let converted = convert_tagged_scalar(tag, &folded)?;
            return Some(Built::converted(converted));
        }
        if folded == "<<" && matches!(tag, Tag::None | Tag::NonSpecific | Tag::Merge) {
            // The plain merge spelling stands for the
            // consumers that can take it as a key to read,
            // its token recorded on the value, and the
            // consumers that stand in value position refuse
            // it on the record.
            return Some(Built {
                value: YamlValue::Text("<<".to_string()),
                tally: 0,
                items: None,
                text: Some("<<".to_string()),
                plain_token: Some("<<".to_string()),
                tag_done: false,
            });
        }
        let value = resolve_plain_scalar(&folded)?;
        Some(Built {
            value,
            tally: 0,
            items: None,
            text: Some(folded),
            plain_token: None,
            tag_done: false,
        })
    }
}

/// Fold the breaks inside a flow plain scalar, the chunks
/// between breaks trimming at their edges and every run of
/// breaks folding by the run rule, one break of line feed
/// kind to one space and one separator standing as itself
/// and a longer run standing as the rule hands it back,
/// blanks standing beside a break standing inside its run.
fn fold_flow_plain(raw: &str) -> String {
    enum Segment {
        Text(String),
        Run(Vec<BreakKind>),
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut text = String::new();
    let mut run: Vec<BreakKind> = Vec::new();
    for ch in raw.chars() {
        match ch {
            '\n' | '\u{2028}' | '\u{2029}' => {
                if !text.is_empty() {
                    segments.push(Segment::Text(std::mem::take(&mut text)));
                }
                run.push(match ch {
                    '\n' => BreakKind::LineFeed,
                    '\u{2028}' => BreakKind::LineSeparator,
                    _ => BreakKind::ParagraphSeparator,
                });
            }
            _ => {
                if !run.is_empty() {
                    segments.push(Segment::Run(std::mem::take(&mut run)));
                }
                text.push(ch);
            }
        }
    }
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    if !run.is_empty() {
        segments.push(Segment::Run(run));
    }
    let mut folded = String::new();
    let mut pending: Vec<BreakKind> = Vec::new();
    let mut seen_text = false;
    for segment in segments {
        match segment {
            Segment::Run(kinds) => pending.extend(kinds),
            Segment::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if seen_text {
                    folded.push_str(&fold_break_run(&pending));
                }
                folded.push_str(trimmed);
                seen_text = true;
                pending.clear();
            }
        }
    }
    folded
}

fn parse_flow_single_quoted(text: &str) -> Option<(String, usize)> {
    match read_quoted_span(text, false) {
        SpanRead::Closed(value, end) => Some((value, text[..end].chars().count())),
        _ => None,
    }
}

fn parse_flow_double_quoted(text: &str) -> Option<(String, usize)> {
    match read_quoted_span(text, true) {
        SpanRead::Closed(value, end) => Some((value, text[..end].chars().count())),
        _ => None,
    }
}
