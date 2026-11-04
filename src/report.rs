//! The report, mirroring the Python report module, its JSON
//! rendering written the way the Python dumps with a two space
//! indent writes it.

use crate::errors::DeadgateError;
use crate::ingest::{Record, Workflow};
use crate::jsonvalue::JsonValue;
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

/// Build the report object, its fields standing in the order
/// the Python mapping builds them.
pub fn build_report(
    contexts: &[String],
    workflows: &[Workflow],
    records: &[Record],
    branch: &str,
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
                                    ("workflow", JsonValue::text(producer.workflow)),
                                    ("file", JsonValue::text(producer.file)),
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
        ("branch", JsonValue::text(branch)),
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

/// The markdown rendering of the report.
pub fn render_markdown(report: &JsonValue) -> String {
    let branch = report
        .get("branch")
        .and_then(|value| value.as_text())
        .unwrap_or("");
    let mut lines: Vec<String> = vec!["# deadgate report".to_string(), String::new()];
    match report.get("window") {
        Some(JsonValue::Object(_)) => {
            let window = report.get("window").expect("window");
            let first = window
                .get("first")
                .and_then(|value| value.as_text())
                .unwrap_or("");
            let last = window
                .get("last")
                .and_then(|value| value.as_text())
                .unwrap_or("");
            lines.push(format!(
                "Branch {branch}. History window {first} to {last}."
            ));
        }
        _ => lines.push(format!("Branch {branch}. History window none.")),
    }
    lines.push(String::new());
    lines.push("| Required check | Producing job | Status | Last ran |".to_string());
    lines.push("| --- | --- | --- | --- |".to_string());
    if let Some(JsonValue::List(checks)) = report.get("checks") {
        for check in checks {
            let context = check
                .get("context")
                .and_then(|value| value.as_text())
                .unwrap_or("");
            let producers = check.get("producers");
            let producing = match producers {
                Some(JsonValue::List(items)) if !items.is_empty() => items
                    .iter()
                    .map(|producer| {
                        let workflow = producer
                            .get("workflow")
                            .and_then(|value| value.as_text())
                            .unwrap_or("");
                        let job_name = producer
                            .get("job_name")
                            .and_then(|value| value.as_text())
                            .unwrap_or("");
                        format!("{workflow} / {job_name}")
                    })
                    .collect::<Vec<String>>()
                    .join(", "),
                _ => "none".to_string(),
            };
            let last_ran = check
                .get("last_run_date")
                .and_then(|value| value.as_text())
                .unwrap_or("never");
            let status_text = check
                .get("status")
                .and_then(|value| value.as_text())
                .unwrap_or("");
            let mut status = status_word(status_text).to_string();
            if let Some(JsonValue::List(notes)) = check.get("notes") {
                if !notes.is_empty() {
                    let words: Vec<&str> = notes.iter().filter_map(|note| note.as_text()).collect();
                    status = format!("{status} ({})", words.join(", "));
                }
            }
            lines.push(format!(
                "| {context} | {producing} | {status} | {last_ran} |"
            ));
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
