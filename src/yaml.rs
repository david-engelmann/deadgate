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
//! position, quoted scalars spanning lines refused, a mapping
//! key that is a container refused, and nesting bounded at
//! 4000. Anchors and aliases and merge keys and tags stand
//! read under the fourth slice inside its borders, properties
//! standing in key position refused beyond the merge and
//! non-specific tags, a tag outside the standing set refused,
//! and alias and merge expansion bounded at 4000 across one
//! document. A scalar
//! the resolver reads as a date or a timestamp whose value does
//! not stand refuses the document, as the Python command dies on
//! it with an uncaught error.

use std::collections::{HashMap, HashSet};

use crate::dates::{civil_from_days, days_from_civil, valid_calendar_date};
use crate::yamlvalue::{YamlKey, YamlValue};

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
        YamlValue::List(_) | YamlValue::Mapping(_) => python_repr(value),
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
    }
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
    Refuse,
}

impl Tag {
    fn is_constructor(&self) -> bool {
        matches!(
            self,
            Tag::Str | Tag::Int | Tag::Float | Tag::Bool | Tag::Null | Tag::Timestamp
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
/// resolves inside it.
#[derive(Clone)]
struct AnchorVal {
    value: YamlValue,
    tally: usize,
    items: Option<Vec<usize>>,
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
#[derive(Clone, Copy, PartialEq)]
enum KeyTag {
    None,
    NonSpecific,
    Merge,
    Refuse,
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
            let tag = match props.tag {
                Tag::None => KeyTag::None,
                Tag::NonSpecific => KeyTag::NonSpecific,
                Tag::Merge => KeyTag::Merge,
                _ => KeyTag::Refuse,
            };
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
        KeySplit::Props(_, offset) => find_key_split(&text[offset..]).is_some(),
        KeySplit::None => find_key_split(text).is_some(),
        KeySplit::Refuse => false,
    }
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
        if *ch == '\n' {
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
                '#' if matches!(previous, None | Some(' ' | '\n' | ',' | '[' | '{')) => {
                    state = State::Comment;
                }
                '\'' if matches!(
                    previous,
                    None | Some(' ' | '\n' | ':' | ',' | '[' | '{' | '-' | '?')
                ) =>
                {
                    state = State::Single;
                }
                '"' if matches!(
                    previous,
                    None | Some(' ' | '\n' | ':' | ',' | '[' | '{' | '-' | '?')
                ) =>
                {
                    state = State::Double;
                }
                _ => {}
            },
            State::Comment => {
                if ch == '\n' {
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

fn parse_yaml_document(text: &str) -> Option<YamlValue> {
    // The runtime reading takes the carriage return forms and
    // the next line and line separator and paragraph separator
    // characters all as line breaks, so the text is normalized
    // to the one break before it is split into lines. The tab
    // standing is taken over the normalized text, so the block
    // content spans the tab pass exempts are found under every
    // break form, the carriage return forms among them, while
    // the printable screen below runs over the text as given.
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{85}', "\n")
        .replace('\u{2028}', "\n")
        .replace('\u{2029}', "\n");
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
    let segments: Vec<&str> = normalized.split('\n').collect();
    let last_segment = segments.len() - 1;
    for (index, raw) in segments.iter().enumerate() {
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
            has_break: index < last_segment,
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
            opener_seen = true;
            let standing = cursor.lines[start].clone();
            cursor.lines[start] = Line {
                indent: 0,
                text: rest.to_string(),
                raw: rest.to_string(),
                has_break: standing.has_break,
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
            if column_zero && (text == "---" || text.starts_with("--- ") || text.starts_with('%')) {
                return None;
            }
            if column_zero && (text == "..." || text.starts_with("... ")) {
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
    Some(value)
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
                let built = read_following_block(cursor, &line, parent_indent, depth, state)?;
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
) -> Option<Built> {
    let Some(nj) = cursor.peek_content() else {
        return Some(Built::empty_scalar());
    };
    let next = cursor.lines[nj].clone();
    if next.indent == line.indent && is_dash_line(&next) {
        parse_sequence(cursor, next.indent, depth + 1, state)
    } else if next.indent >= line.indent || next.indent > parent_indent {
        parse_block(cursor, parent_indent, depth + 1, state)
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
) -> Option<Built> {
    match cursor.peek_content() {
        None => Some(Built::empty_scalar()),
        Some(nj) => {
            let next = cursor.lines[nj].clone();
            if next.indent > indent {
                parse_block(cursor, indent, depth + 1, state)
            } else if next.indent == indent && is_dash_line(&next) {
                parse_sequence(cursor, indent, depth + 1, state)
            } else {
                Some(Built::empty_scalar())
            }
        }
    }
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
        if key_props.tag == KeyTag::Refuse {
            return None;
        }
        let first_pair = explicit.is_empty() && merge_sources.is_empty();
        if key_props.anchor.is_some() && !first_pair {
            return None;
        }
        let (key_raw, rest_start) = find_key_split(&line.text[key_offset..])?;
        let rest_start = key_offset + rest_start;
        let key_value = parse_key_scalar(&key_raw)?;
        if let Some(name) = &key_props.anchor {
            // The anchor standing in front of a block mapping
            // binds the first key scalar of the mapping, not
            // the mapping itself, and a name bound anywhere in
            // the document refuses a second binding.
            if state.bound.contains_key(name) || state.building.contains(name) {
                return None;
            }
            state.bound.insert(
                name.clone(),
                AnchorVal {
                    value: key_value.clone(),
                    tally: 0,
                    items: None,
                },
            );
        }
        let key = YamlKey::of(&key_value)?;
        let merge_key = key_props.tag == KeyTag::Merge
            || (matches!(key_props.tag, KeyTag::None | KeyTag::NonSpecific) && key_raw == "<<");
        let rest = line.text[rest_start..].trim_start().to_string();
        cursor.pos = li + 1;
        let value = if rest.is_empty() || rest.starts_with('#') {
            following_block_value(cursor, indent, depth, state)?
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
                    let built = following_block_value(cursor, indent, depth, state)?;
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
                            parse_block(cursor, indent, depth + 1, state)?
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
            cursor.lines[li] = Line {
                indent: virtual_indent,
                text: content.clone(),
                raw: content,
                has_break: standing_break,
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

/// Parse a single quoted scalar standing on one line, handing
/// back its text and the byte index just past its close.
fn parse_single_quoted(text: &str) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut index = 1;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'\'' {
            if bytes.get(index + 1) == Some(&b'\'') {
                out.push('\'');
                index += 2;
                continue;
            }
            return Some((out, index + 1));
        }
        let ch = text[index..].chars().next()?;
        out.push(ch);
        index += ch.len_utf8();
    }
    None
}

/// Parse a double quoted scalar standing on one line, handing
/// back its text and the byte index just past its close.
fn parse_double_quoted(text: &str) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut index = 1;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some((out, index + 1)),
            b'\\' => {
                index += 1;
                let escape = *bytes.get(index)?;
                index += 1;
                match escape {
                    b'0' => out.push('\0'),
                    b'a' => out.push('\u{7}'),
                    b'b' => out.push('\u{8}'),
                    b't' | b'\t' => out.push('\t'),
                    b'n' => out.push('\n'),
                    b'v' => out.push('\u{B}'),
                    b'f' => out.push('\u{C}'),
                    b'r' => out.push('\r'),
                    b'e' => out.push('\u{1B}'),
                    b'"' => out.push('"'),
                    b'/' => out.push('/'),
                    b'\\' => out.push('\\'),
                    b'N' => out.push('\u{85}'),
                    b'_' => out.push('\u{A0}'),
                    b'L' => out.push('\u{2028}'),
                    b'P' => out.push('\u{2029}'),
                    b'x' | b'u' | b'U' => {
                        let count = match escape {
                            b'x' => 2,
                            b'u' => 4,
                            _ => 8,
                        };
                        let mut value: u32 = 0;
                        for _ in 0..count {
                            let digit = bytes.get(index)?;
                            let digit = (*digit as char).to_digit(16)?;
                            value = value * 16 + digit;
                            index += 1;
                        }
                        out.push(char::from_u32(value)?);
                    }
                    _ => return None,
                }
            }
            _ => {
                let ch = text[index..].chars().next()?;
                if (ch as u32) < 0x20 && ch != '\t' {
                    return None;
                }
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    None
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
                    read_following_block_inline(cursor, block_indent, parent_indent, depth, state)?
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
) -> Option<Built> {
    let Some(nj) = cursor.peek_content() else {
        return Some(Built::empty_scalar());
    };
    let next = cursor.lines[nj].clone();
    if next.indent == block_indent && is_dash_line(&next) {
        parse_sequence(cursor, block_indent, depth + 1, state)
    } else if next.indent > parent_indent {
        parse_block(cursor, parent_indent, depth + 1, state)
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
            while !flow_closed(&combined) {
                if cursor.pos >= cursor.lines.len() {
                    return None;
                }
                let line = cursor.lines[cursor.pos].clone();
                cursor.pos += 1;
                combined.push('\n');
                combined.push_str(&" ".repeat(line.indent));
                combined.push_str(&line.text);
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
            let (value, end) = parse_double_quoted(text)?;
            check_trailing(&text[end..])?;
            if tag.is_constructor() {
                let converted = convert_tagged_scalar(tag, &value)?;
                return Some(Built::converted(converted));
            }
            Some(Built::from_text(value))
        }
        '\'' => {
            let (value, end) = parse_single_quoted(text)?;
            check_trailing(&text[end..])?;
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
            let mut gaps: Vec<usize> = Vec::new();
            let mut scan = cursor.pos;
            let mut pending_blanks = 0usize;
            let mut comment_seen = false;
            loop {
                let Some(line) = cursor.lines.get(scan) else {
                    break;
                };
                let line = line.clone();
                let trimmed = line.text.trim();
                if trimmed.is_empty() {
                    pending_blanks += 1;
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
                gaps.push(pending_blanks);
                chunks.push(segment.trim().to_string());
                pending_blanks = 0;
                scan += 1;
                cursor.pos = scan;
            }
            let mut folded = chunks[0].clone();
            for (gap, chunk) in gaps.iter().zip(chunks.iter().skip(1)) {
                if *gap == 0 {
                    folded.push(' ');
                } else {
                    folded.push_str(&"\n".repeat(*gap));
                }
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
    let mut raw_lines: Vec<(String, bool)> = Vec::new();
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
                raw_lines.push((line.raw.clone(), line.has_break));
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
            raw_lines.push((line.raw.clone(), line.has_break));
            scan += 1;
            continue;
        }
        let width = block_indent.expect("decided width");
        if is_blank {
            raw_lines.push((line.raw.clone(), line.has_break));
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
        raw_lines.push((line.raw.clone(), line.has_break));
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
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for (raw, has_break) in &raw_lines {
        let spaces = raw.chars().take_while(|ch| *ch == ' ').count();
        let all_spaces = raw.chars().all(|ch| ch == ' ');
        if undecided || (all_spaces && spaces <= width) {
            pieces.push(Piece {
                content: String::new(),
                blank: true,
                more: false,
                has_break: *has_break,
            });
        } else {
            let content: String = raw.chars().skip(width).collect();
            let more = spaces > width || content.starts_with('\t');
            pieces.push(Piece {
                content,
                blank: false,
                more,
                has_break: *has_break,
            });
        }
    }
    // Assemble the value before its final break, counting the
    // breaks that stand at its end for the chomping.
    let first = pieces.iter().position(|piece| !piece.blank);
    let last = pieces.iter().rposition(|piece| !piece.blank);
    let (body, trailing) = match (first, last) {
        (Some(first), Some(last)) => {
            // Blank lines standing before the content count
            // one break each in the value, under every
            // chomping, once content follows.
            let mut body = "\n".repeat(first);
            for index in first..=last {
                if index > first {
                    let previous = &pieces[index - 1];
                    let current = &pieces[index];
                    if !folded {
                        body.push('\n');
                    } else if previous.blank || current.blank {
                        let blanks = {
                            let mut count = 0;
                            let mut walk = index - 1;
                            loop {
                                if pieces[walk].blank {
                                    count += 1;
                                    if walk == first {
                                        break;
                                    }
                                    walk -= 1;
                                } else {
                                    break;
                                }
                            }
                            count
                        };
                        if current.blank {
                            // Breaks inside a blank run are
                            // counted at the run's last blank.
                        } else if blanks > 0 {
                            // A blank run between two content
                            // lines gives one break per blank
                            // line, and where the line before
                            // the run or the line after it is
                            // more indented the break of the
                            // line before the run stands as
                            // well, since a break beside a
                            // more indented line is never
                            // folded away.
                            let before = &pieces[index - 1 - blanks];
                            let kept = if before.more || current.more { 1 } else { 0 };
                            body.push_str(&"\n".repeat(blanks + kept));
                        } else {
                            body.push('\n');
                        }
                        if current.blank && index == last {
                            body.push('\n');
                        }
                    } else if previous.more || current.more {
                        body.push('\n');
                    } else {
                        body.push(' ');
                    }
                }
                body.push_str(&pieces[index].content);
            }
            let last_piece = &pieces[last];
            let mut trailing = if last_piece.has_break { 1usize } else { 0usize };
            for piece in &pieces[last + 1..] {
                if piece.has_break {
                    trailing += 1;
                }
            }
            (body, trailing)
        }
        _ => {
            let mut breaks = 0usize;
            for piece in &pieces {
                if piece.has_break {
                    breaks += 1;
                }
            }
            (String::new(), breaks)
        }
    };
    let value = match chomping {
        Chomping::Strip => body,
        Chomping::Clip => {
            if body.is_empty() {
                body
            } else if trailing > 0 {
                format!("{body}\n")
            } else {
                body
            }
        }
        Chomping::Keep => {
            if body.is_empty() {
                "\n".repeat(trailing)
            } else {
                format!("{body}{}", "\n".repeat(trailing))
            }
        }
    };
    Some(YamlValue::Text(value))
}

/// Whether the brackets of a flow text stand balanced, quotes
/// taken into account.
fn flow_closed(text: &str) -> bool {
    let mut depth = 0i64;
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match quote {
            Some('"') => match ch {
                '\\' => {
                    chars.next();
                }
                '"' => quote = None,
                _ => {}
            },
            Some('\'') => {
                if ch == '\'' {
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                    } else {
                        quote = None;
                    }
                }
            }
            _ => match ch {
                '"' | '\'' => quote = Some(ch),
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return true;
                    }
                }
                '#' => {
                    // A comment runs to the end of its line.
                    for next in chars.by_ref() {
                        if next == '\n' {
                            break;
                        }
                    }
                }
                _ => {}
            },
        }
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
                Some(' ') | Some('\t') | Some('\n') | Some('\r') => {
                    self.pos += 1;
                }
                Some('#') => {
                    while let Some(ch) = self.chars.get(self.pos) {
                        self.pos += 1;
                        if *ch == '\n' {
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
                        None | Some(' ' | '\t' | '\n' | '\r' | ',' | ']' | '}' | ':') => {}
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
            && !matches!(self.chars[end], ' ' | '\t' | '\n' | '\r' | ',' | ']' | '}')
        {
            end += 1;
        }
        let token: String = self.chars[self.pos..end].iter().collect();
        self.pos = end;
        resolve_tag_spelling(&token, &self.state.bindings)
    }

    /// Take the one tag token a flow key can carry, the merge
    /// tag and the non-specific tag standing and every other
    /// tag refusing.
    fn take_key_tag(&mut self) -> Option<KeyTag> {
        match self.take_tag_token()? {
            Tag::Merge => Some(KeyTag::Merge),
            Tag::NonSpecific => Some(KeyTag::NonSpecific),
            _ => None,
        }
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
                            if item_anchor {
                                return None;
                            }
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
                            if item_anchor || !matches!(item_tag, Tag::None | Tag::NonSpecific) {
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
                    // its own that no property reaches, an
                    // anchor or an alias in front of it
                    // refusing and a tag standing only as the
                    // merge tag or the non-specific tag.
                    if self.peek() == Some('&') || self.peek() == Some('*') {
                        return None;
                    }
                    let mut key_tag = KeyTag::None;
                    if self.peek() == Some('!') {
                        key_tag = self.take_key_tag()?;
                        self.skip_separators();
                        if self.peek() == Some('&') || self.peek() == Some('*') {
                            return None;
                        }
                    }
                    let (key_value, key_plain) = match self.peek()? {
                        '"' => {
                            let text: String = self.chars[self.pos..].iter().collect();
                            let (value, end) = parse_flow_double_quoted(&text)?;
                            self.pos += end;
                            (YamlValue::Text(value), None)
                        }
                        '\'' => {
                            let text: String = self.chars[self.pos..].iter().collect();
                            let (value, end) = parse_flow_single_quoted(&text)?;
                            self.pos += end;
                            (YamlValue::Text(value), None)
                        }
                        '[' | '{' => {
                            let built = self.parse_value(depth + 1)?;
                            (built.value, None)
                        }
                        ',' | '}' | ':' | '|' | '>' | '%' | '@' | '`' => return None,
                        _ => {
                            let folded = self.scan_plain_token()?;
                            let plain = if folded == "<<" {
                                Some(folded.clone())
                            } else {
                                None
                            };
                            (resolve_plain_scalar(&folded)?, plain)
                        }
                    };
                    self.skip_separators();
                    let merge_key = key_tag == KeyTag::Merge
                        || (matches!(key_tag, KeyTag::None | KeyTag::NonSpecific)
                            && key_plain.as_deref() == Some("<<"));
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
                    if merge_key {
                        merge_sources.push(value);
                    } else {
                        let key = YamlKey::of(&key_value)?;
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
                        || matches!(next, Some(' ' | '\t' | '\n' | ',' | ']' | '}' | '[' | '{'))
                    {
                        break;
                    }
                    index += 1;
                }
                Some('#') => {
                    if index > start && matches!(self.chars[index - 1], ' ' | '\t' | '\n') {
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

/// Fold the line breaks inside a flow plain scalar, a single
/// break standing as a space and each further break standing
/// as one more line break.
fn fold_flow_plain(raw: &str) -> String {
    let mut out = String::new();
    let chunks = raw.split('\n');
    let mut first = true;
    let mut pending_newlines = 0usize;
    for chunk in chunks {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            pending_newlines += 1;
            continue;
        }
        if !first {
            if pending_newlines <= 1 {
                out.push(' ');
            } else {
                out.push_str(&"\n".repeat(pending_newlines - 1));
            }
        }
        out.push_str(trimmed);
        first = false;
        pending_newlines = 1;
    }
    out
}

fn parse_flow_single_quoted(text: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<char> = Vec::new();
    let mut index = 1;
    while index < chars.len() {
        match chars[index] {
            '\'' => {
                if chars.get(index + 1) == Some(&'\'') {
                    out.push('\'');
                    index += 2;
                    continue;
                }
                return Some((fold_quoted_chars(&out), index + 1));
            }
            ch => {
                out.push(ch);
                index += 1;
            }
        }
    }
    None
}

fn parse_flow_double_quoted(text: &str) -> Option<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<char> = Vec::new();
    let mut index = 1;
    while index < chars.len() {
        match chars[index] {
            '"' => return Some((fold_quoted_chars(&out), index + 1)),
            '\\' => {
                index += 1;
                let escape = *chars.get(index)?;
                index += 1;
                if escape == '\n' {
                    // A broken line inside a double quoted
                    // scalar joins with nothing standing.
                    while matches!(chars.get(index), Some(' ' | '\t')) {
                        index += 1;
                    }
                    continue;
                }
                let ch = match escape {
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
                            let digit = chars.get(index)?;
                            let digit = digit.to_digit(16)?;
                            value = value * 16 + digit;
                            index += 1;
                        }
                        char::from_u32(value)?
                    }
                    _ => return None,
                };
                out.push(ch);
            }
            ch => {
                if (ch as u32) < 0x20 && ch != '\t' && ch != '\n' {
                    return None;
                }
                out.push(ch);
                index += 1;
            }
        }
    }
    None
}

/// Fold the line breaks gathered inside a quoted scalar that
/// spans lines, as the folding of a plain scalar folds them.
fn fold_quoted_chars(chars: &[char]) -> String {
    let text: String = chars.iter().collect();
    let mut out = String::new();
    let mut first = true;
    let mut pending_newlines = 0usize;
    for chunk in text.split('\n') {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            pending_newlines += 1;
            continue;
        }
        if !first {
            if pending_newlines <= 1 {
                out.push(' ');
            } else {
                out.push_str(&"\n".repeat(pending_newlines - 1));
            }
        } else if pending_newlines > 0 {
            out.push_str(&"\n".repeat(pending_newlines));
        }
        out.push_str(trimmed);
        first = false;
        pending_newlines = 1;
    }
    if pending_newlines > 1 && !first {
        out.push_str(&"\n".repeat(pending_newlines - 1));
    }
    out
}
