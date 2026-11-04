//! The YAML reading, mirroring the PyYAML safe load of the
//! runtime this repo runs on, and the Python text forms of the
//! values it produces.
//!
//! The reading covers block mappings and sequences, flow
//! collections, plain scalars with their continuation folding,
//! and quoted scalars standing on one line, and it resolves
//! plain scalars with the resolver of the runtime, the YAML 1.1
//! spellings standing as the probe recorded them. The limits the
//! contract states stand at the edges, anchors and aliases and
//! merge keys and tags and block scalars and document apparatus
//! beyond one leading opener and one trailing closer refused,
//! quoted scalars spanning lines refused, a mapping key that is
//! a container refused, and nesting bounded at 4000. A scalar
//! the resolver reads as a date or a timestamp whose value does
//! not stand refuses the document, as the Python command dies on
//! it with an uncaught error.

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

/// Whether a tab stands anywhere the runtime scanner refuses
/// one, which is everywhere outside a quoted scalar and a
/// comment, as the tab probe recorded.
fn tab_refused(text: &str) -> bool {
    #[derive(PartialEq)]
    enum State {
        Normal,
        Single,
        Double,
        Comment,
    }
    let mut state = State::Normal;
    let mut previous: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match state {
            State::Normal => match ch {
                '\t' => return true,
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
                    if chars.peek() == Some(&'\'') {
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
    tag_handles: &mut Vec<String>,
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
            let digits = |part: &str| {
                !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())
            };
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
            if tag_handles.iter().any(|standing| standing == args[0]) {
                return false;
            }
            tag_handles.push(args[0].to_string());
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
    if tab_refused(text) {
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
    // The runtime reading takes the carriage return forms and
    // the next line and line separator and paragraph separator
    // characters all as line breaks, so the text is normalized
    // to the one break before it is split into lines.
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{85}', "\n")
        .replace('\u{2028}', "\n")
        .replace('\u{2029}', "\n");
    let mut lines: Vec<Line> = Vec::new();
    for raw in normalized.split('\n') {
        let mut indent = 0usize;
        let mut tabbed = false;
        let mut content_start = raw.len();
        for (index, ch) in raw.char_indices() {
            match ch {
                ' ' => indent += 1,
                '\t' => tabbed = true,
                _ => {
                    content_start = index;
                    break;
                }
            }
        }
        let content = &raw[content_start.min(raw.len())..];
        if tabbed && !content.trim().is_empty() {
            return None;
        }
        lines.push(Line {
            indent,
            text: content.to_string(),
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
    let mut tag_handles: Vec<String> = Vec::new();
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
            cursor.lines[start] = Line {
                indent: 0,
                text: rest.to_string(),
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
            if text == "---" || text.starts_with("--- ") || text.starts_with('%') {
                return None;
            }
            if text == "..." || text.starts_with("... ") {
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
    let value = parse_block(&mut cursor, 0)?;
    if cursor.peek_content().is_some() {
        return None;
    }
    Some(value)
}

fn parse_block(cursor: &mut Cursor, depth: usize) -> Option<YamlValue> {
    if depth > MAX_YAML_NESTING {
        return None;
    }
    let li = cursor.peek_content()?;
    let line = cursor.lines[li].clone();
    if is_dash_line(&line) {
        parse_sequence(cursor, line.indent, depth)
    } else if find_key_split(&line.text).is_some() {
        parse_mapping(cursor, line.indent, depth)
    } else {
        cursor.pos = li + 1;
        parse_inline_value(&line.text, cursor, line.indent, depth)
    }
}

fn parse_mapping(cursor: &mut Cursor, indent: usize, depth: usize) -> Option<YamlValue> {
    let mut pairs: Vec<(YamlKey, YamlValue)> = Vec::new();
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
        if is_dash_line(&line) {
            break;
        }
        let (key_raw, rest_start) = find_key_split(&line.text)?;
        let key_value = parse_key_scalar(&key_raw)?;
        let key = YamlKey::of(&key_value)?;
        let rest = line.text[rest_start..].trim_start().to_string();
        cursor.pos = li + 1;
        let value = if rest.is_empty() || rest.starts_with('#') {
            match cursor.peek_content() {
                None => YamlValue::Null,
                Some(nj) => {
                    let next = cursor.lines[nj].clone();
                    if next.indent > indent {
                        parse_block(cursor, depth + 1)?
                    } else if next.indent == indent && is_dash_line(&next) {
                        parse_sequence(cursor, indent, depth + 1)?
                    } else {
                        YamlValue::Null
                    }
                }
            }
        } else {
            parse_inline_value(&rest, cursor, indent, depth + 1)?
        };
        insert_pair(&mut pairs, key, value);
    }
    Some(YamlValue::Mapping(pairs))
}

fn parse_sequence(cursor: &mut Cursor, indent: usize, depth: usize) -> Option<YamlValue> {
    let mut items = Vec::new();
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
        let after = &line.text[1..];
        let content = after.trim_start().to_string();
        if content.is_empty() || content.starts_with('#') {
            cursor.pos = li + 1;
            let value = match cursor.peek_content() {
                None => YamlValue::Null,
                Some(nj) => {
                    let next = cursor.lines[nj].clone();
                    if next.indent > indent {
                        parse_block(cursor, depth + 1)?
                    } else {
                        YamlValue::Null
                    }
                }
            };
            items.push(value);
            continue;
        }
        let lead = after.len() - after.trim_start().len();
        let virtual_indent = indent + 1 + lead;
        if is_dash_text(&content) || find_key_split(&content).is_some() {
            cursor.lines[li] = Line {
                indent: virtual_indent,
                text: content,
            };
            cursor.pos = li;
            let value = parse_block(cursor, depth + 1)?;
            items.push(value);
        } else {
            cursor.pos = li + 1;
            let value = parse_inline_value(&content, cursor, indent, depth + 1)?;
            items.push(value);
        }
    }
    Some(YamlValue::List(items))
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
    depth: usize,
) -> Option<YamlValue> {
    let first = text.chars().next()?;
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
            let mut parser = FlowParser::new(&combined);
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
            Some(YamlValue::Text(value))
        }
        '\'' => {
            let (value, end) = parse_single_quoted(text)?;
            check_trailing(&text[end..])?;
            Some(YamlValue::Text(value))
        }
        '&' | '*' | '!' | '|' | '>' | '%' | '@' | '`' | ']' | '}' | ',' => None,
        '?' if text == "?" || text.starts_with("? ") => None,
        ':' if text == ":" || text.starts_with(": ") => None,
        '-' if text == "-" || text.starts_with("- ") => None,
        '#' => Some(YamlValue::Null),
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
            resolve_plain_scalar(&folded)
        }
    }
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

struct FlowParser {
    chars: Vec<char>,
    pos: usize,
}

impl FlowParser {
    fn new(text: &str) -> FlowParser {
        FlowParser {
            chars: text.chars().collect(),
            pos: 0,
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

    fn parse_value(&mut self, depth: usize) -> Option<YamlValue> {
        if depth > MAX_YAML_NESTING {
            return None;
        }
        self.skip_separators();
        match self.peek()? {
            '[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_separators();
                    if self.peek() == Some(']') {
                        self.pos += 1;
                        return Some(YamlValue::List(items));
                    }
                    let item = self.parse_value(depth + 1)?;
                    self.skip_separators();
                    let item = if self.peek() == Some(':') {
                        // A single pair mapping standing in a
                        // sequence.
                        let key = YamlKey::of(&item)?;
                        self.pos += 1;
                        self.skip_separators();
                        let value = match self.peek() {
                            Some(',') | Some(']') => YamlValue::Null,
                            _ => self.parse_value(depth + 1)?,
                        };
                        YamlValue::Mapping(vec![(key, value)])
                    } else {
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
                            return Some(YamlValue::List(items));
                        }
                        _ => return None,
                    }
                }
            }
            '{' => {
                self.pos += 1;
                let mut pairs: Vec<(YamlKey, YamlValue)> = Vec::new();
                loop {
                    self.skip_separators();
                    if self.peek() == Some('}') {
                        self.pos += 1;
                        return Some(YamlValue::Mapping(pairs));
                    }
                    let key_item = self.parse_value(depth + 1)?;
                    let key = YamlKey::of(&key_item)?;
                    self.skip_separators();
                    let value = if self.peek() == Some(':') {
                        self.pos += 1;
                        self.skip_separators();
                        match self.peek() {
                            Some(',') | Some('}') => YamlValue::Null,
                            _ => self.parse_value(depth + 1)?,
                        }
                    } else {
                        YamlValue::Null
                    };
                    insert_pair(&mut pairs, key, value);
                    self.skip_separators();
                    match self.peek() {
                        Some(',') => {
                            self.pos += 1;
                        }
                        Some('}') => {
                            self.pos += 1;
                            return Some(YamlValue::Mapping(pairs));
                        }
                        _ => return None,
                    }
                }
            }
            '"' => {
                let text: String = self.chars[self.pos..].iter().collect();
                let (value, end) = parse_flow_double_quoted(&text)?;
                self.pos += end;
                Some(YamlValue::Text(value))
            }
            '\'' => {
                let text: String = self.chars[self.pos..].iter().collect();
                let (value, end) = parse_flow_single_quoted(&text)?;
                self.pos += end;
                Some(YamlValue::Text(value))
            }
            ',' | ']' | '}' | ':' => None,
            _ => {
                // A token opening with an indicator stands in
                // the refused family of the contract, an anchor
                // or an alias or a tag or a block scalar head or
                // a reserved indicator, in flow position as in
                // block position. The plain reading never takes
                // one as text.
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
                                    Some(' ' | '\t' | '\n' | ',' | ']' | '}' | '[' | '{')
                                )
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
                resolve_plain_scalar(&folded)
            }
        }
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
