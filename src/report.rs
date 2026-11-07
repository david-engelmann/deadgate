//! The report, mirroring the Python report module, its JSON
//! rendering written the way the Python dumps with a two space
//! indent writes it.

use crate::errors::DeadgateError;
use crate::ingest::{Record, Workflow};
use crate::jsonvalue::JsonValue;
use crate::pytext::PyText;
use crate::reconcile::reconcile;

pub const FORMAT: &str = "deadgate-report/1";

fn status_word(status: &str) -> &str {
    match status {
        "alive" => "alive",
        "no_producer" => "no producer",
        "never_run" => "never ran",
        "skipped_only" => "skipped only",
        _ => status,
    }
}

/// The report node a text in point form stands as, a plain
/// string node when the text is plain and a point node when it
/// carries an escape point, so its spelling is settled at the
/// rendering and not before.
fn text_node(text: &PyText) -> JsonValue {
    match text.plain() {
        Some(plain) => JsonValue::Text(plain),
        None => JsonValue::PointText(text.clone()),
    }
}

/// Build the report object, its fields standing in the order
/// the Python mapping builds them.
pub fn build_report(
    contexts: &[String],
    workflows: &[Workflow],
    records: &[Record],
    branch: &PyText,
    window: Option<JsonValue>,
) -> Result<JsonValue, DeadgateError> {
    let checks = reconcile(contexts, workflows, records, branch)?;
    let alive = checks
        .iter()
        .filter(|check| check.status == "alive")
        .count();
    let check_values: Vec<JsonValue> = checks
        .into_iter()
        .map(|check| {
            JsonValue::object(vec![
                ("context", JsonValue::text(check.context)),
                ("status", JsonValue::text(check.status)),
                (
                    "producers",
                    JsonValue::List(
                        check
                            .producers
                            .into_iter()
                            .map(|producer| {
                                JsonValue::object(vec![
                                    ("workflow", text_node(&producer.workflow)),
                                    ("file", text_node(&producer.file)),
                                    ("job", JsonValue::text(producer.job)),
                                    ("job_name", JsonValue::text(producer.job_name)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "last_run_date",
                    match check.last_run_date {
                        Some(date) => JsonValue::text(date),
                        None => JsonValue::Null,
                    },
                ),
                (
                    "notes",
                    JsonValue::List(check.notes.into_iter().map(JsonValue::text).collect()),
                ),
            ])
        })
        .collect();
    let required = check_values.len();
    Ok(JsonValue::object(vec![
        ("format", JsonValue::text(FORMAT)),
        ("branch", text_node(branch)),
        ("window", window.unwrap_or(JsonValue::Null)),
        ("checks", JsonValue::List(check_values)),
        (
            "summary",
            JsonValue::object(vec![
                ("required", JsonValue::Int(required as i64)),
                ("alive", JsonValue::Int(alive as i64)),
                ("paper", JsonValue::Int((required - alive) as i64)),
            ]),
        ),
    ]))
}

/// Escape one string the way the Python JSON encoder escapes
/// it, every character outside the printable ASCII standing as
/// a unicode escape with lowercase hex and an astral character
/// standing as a surrogate pair.
fn escape_json_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{C}' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 || (0x7F..=0xFFFF).contains(&(ch as u32)) => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch if (ch as u32) > 0xFFFF => {
                let mut buffer = [0u16; 2];
                for unit in ch.encode_utf16(&mut buffer) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// Escape one text in point form the way the Python JSON
/// encoder escapes the string the runtime decoded, point by
/// point under the same rules as the plain string escape, so
/// an escape point stands as its Unicode escape and an astral
/// point as a surrogate pair.
fn escape_json_points(points: &[u32], out: &mut String) {
    out.push('"');
    for point in points {
        let point = *point;
        match point {
            0x22 => out.push_str("\\\""),
            0x5C => out.push_str("\\\\"),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x08 => out.push_str("\\b"),
            0x0C => out.push_str("\\f"),
            0x00..=0x1F | 0x7F..=0xFFFF => {
                out.push_str(&format!("\\u{point:04x}"));
            }
            0x10000..=0x10FFFF => {
                let ch = char::from_u32(point).expect("astral point");
                let mut buffer = [0u16; 2];
                for unit in ch.encode_utf16(&mut buffer) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            _ => {
                let ch = char::from_u32(point).expect("scalar point");
                out.push(ch);
            }
        }
    }
    out.push('"');
}

fn write_json(value: &JsonValue, out: &mut String, level: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        JsonValue::Int(integer) => out.push_str(&integer.to_string()),
        JsonValue::BigInt(text) => out.push_str(text),
        JsonValue::Float(number) => {
            if number.is_nan() {
                out.push_str("NaN");
            } else if number.is_infinite() {
                out.push_str(if *number < 0.0 {
                    "-Infinity"
                } else {
                    "Infinity"
                });
            } else {
                out.push_str(&crate::yaml::python_float_text(*number));
            }
        }
        JsonValue::Text(text) => escape_json_string(text, out),
        JsonValue::PointText(text) => escape_json_points(text.points(), out),
        JsonValue::List(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                out.push_str(&"  ".repeat(level + 1));
                write_json(item, out, level + 1);
                if index + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&"  ".repeat(level));
            out.push(']');
        }
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, item)) in pairs.iter().enumerate() {
                out.push_str(&"  ".repeat(level + 1));
                escape_json_string(key, out);
                out.push_str(": ");
                write_json(item, out, level + 1);
                if index + 1 < pairs.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&"  ".repeat(level));
            out.push('}');
        }
    }
}

/// The JSON rendering of the report, with its trailing newline.
pub fn render_json(report: &JsonValue) -> String {
    let mut out = String::new();
    write_json(report, &mut out, 0);
    out.push('\n');
    out
}

/// The bytes one report field stands as in the markdown
/// rendering, a plain string as its UTF-8 and a text in point
/// form as the bytes the runtime writes, an escape point
/// standing back as the byte it came from.
fn field_bytes(value: Option<&JsonValue>) -> Vec<u8> {
    match value {
        Some(JsonValue::Text(text)) => text.clone().into_bytes(),
        Some(JsonValue::PointText(text)) => text.stdout_bytes(),
        _ => Vec::new(),
    }
}

/// The markdown rendering of the report, as bytes, because a
/// field in point form can carry a byte no string holds.
pub fn render_markdown(report: &JsonValue) -> Vec<u8> {
    let branch = field_bytes(report.get("branch"));
    let mut lines: Vec<Vec<u8>> = vec![b"# deadgate report".to_vec(), Vec::new()];
    match report.get("window") {
        Some(JsonValue::Object(_)) => {
            let window = report.get("window").expect("window");
            let first = field_bytes(window.get("first"));
            let last = field_bytes(window.get("last"));
            let mut line = b"Branch ".to_vec();
            line.extend_from_slice(&branch);
            line.extend_from_slice(b". History window ");
            line.extend_from_slice(&first);
            line.extend_from_slice(b" to ");
            line.extend_from_slice(&last);
            line.push(b'.');
            lines.push(line);
        }
        _ => {
            let mut line = b"Branch ".to_vec();
            line.extend_from_slice(&branch);
            line.extend_from_slice(b". History window none.");
            lines.push(line);
        }
    }
    lines.push(Vec::new());
    lines.push(b"| Required check | Producing job | Status | Last ran |".to_vec());
    lines.push(b"| --- | --- | --- | --- |".to_vec());
    if let Some(JsonValue::List(checks)) = report.get("checks") {
        for check in checks {
            let context = field_bytes(check.get("context"));
            let producers = check.get("producers");
            let producing = match producers {
                Some(JsonValue::List(items)) if !items.is_empty() => {
                    let mut joined: Vec<u8> = Vec::new();
                    for (index, producer) in items.iter().enumerate() {
                        if index > 0 {
                            joined.extend_from_slice(b", ");
                        }
                        joined.extend_from_slice(&field_bytes(producer.get("workflow")));
                        joined.extend_from_slice(b" / ");
                        joined.extend_from_slice(&field_bytes(producer.get("job_name")));
                    }
                    joined
                }
                _ => b"none".to_vec(),
            };
            let last_ran = match check.get("last_run_date") {
                Some(JsonValue::Text(text)) => text.clone().into_bytes(),
                _ => b"never".to_vec(),
            };
            let status_text = check
                .get("status")
                .and_then(|value| value.as_text())
                .unwrap_or("");
            let mut status = status_word(status_text).to_string().into_bytes();
            if let Some(JsonValue::List(notes)) = check.get("notes") {
                if !notes.is_empty() {
                    let words: Vec<&str> = notes.iter().filter_map(|note| note.as_text()).collect();
                    let mut widened = status;
                    widened.extend_from_slice(b" (");
                    widened.extend_from_slice(words.join(", ").as_bytes());
                    widened.push(b')');
                    status = widened;
                }
            }
            let mut line = b"| ".to_vec();
            line.extend_from_slice(&context);
            line.extend_from_slice(b" | ");
            line.extend_from_slice(&producing);
            line.extend_from_slice(b" | ");
            line.extend_from_slice(&status);
            line.extend_from_slice(b" | ");
            line.extend_from_slice(&last_ran);
            line.extend_from_slice(b" |");
            lines.push(line);
        }
    }
    let mut out: Vec<u8> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(line);
    }
    out.push(b'\n');
    out
}
