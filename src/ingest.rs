//! Ingestion of the three local inputs, mirroring the Python
//! ingestion module obligation by obligation, the duplicate key
//! refusal of R2-1 standing at the two JSON doors.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::dates::{date_text_of_instant, parse_timestamp_micros};
use crate::errors::DeadgateError;
use crate::jsonvalue::{parse_json_refusing, refusal_message, JsonValue};
use crate::pytext::{path_text, PyText};
use crate::yaml::{parse_yaml, python_str};
use crate::yamlvalue::{key_to_value, YamlKey, YamlValue};

/// One workflow job, as the reconciliation reads it.
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub job: String,
    pub job_name: String,
    pub has_if: bool,
    pub combinations: Option<Vec<String>>,
}

/// One workflow definition. The workflow name and the file
/// name stand in the runtime point form, because a file name
/// off the disk is a run of bytes the runtime decodes with its
/// escape standing, and the name fallback is read off it.
#[derive(Debug, Clone)]
pub struct Workflow {
    pub workflow: PyText,
    pub file: PyText,
    pub triggers: YamlValue,
    /// The jobs keyed by the string form of their identifiers,
    /// in file order.
    pub jobs: Vec<(String, JobInfo)>,
}

/// One execution history record. The workflow name, the status,
/// and the conclusion stand as the values the export carries.
#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub workflow_name: JsonValue,
    pub status: JsonValue,
    pub conclusion: JsonValue,
    pub started_at: Option<i128>,
    pub completed_at: Option<i128>,
}

fn read_json_document(path: &Path, label: &str) -> Result<JsonValue, DeadgateError> {
    let shown = path_text(path);
    let bytes = std::fs::read(path)
        .map_err(|_| DeadgateError::new(format!("{label} cannot be read at {shown}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| DeadgateError::new(format!("{label} at {shown} is not valid JSON")))?;
    parse_json_refusing(&text, label, &shown)
        .map_err(|failure| DeadgateError::new(refusal_message(&failure, label, &shown)))
}

/// The required status check contexts of one branch, in file
/// order, deduplicated at their first standing.
pub fn load_protection(path: &Path) -> Result<Vec<String>, DeadgateError> {
    let shown = path_text(path);
    let data = read_json_document(path, "protection export")?;
    let required = data
        .get("required_status_checks")
        .filter(|value| value.is_object());
    let required = match required {
        Some(value) => value.clone(),
        None => {
            return Err(DeadgateError::new(format!(
                "protection export at {shown} carries no required status checks"
            )))
        }
    };
    let checks = required.get("checks").cloned();
    if let Some(checks_value) = &checks {
        if !matches!(checks_value, JsonValue::Null) && checks_value.as_list().is_none() {
            return Err(DeadgateError::new(format!(
                "protection export at {shown} carries a checks value that is not a list"
            )));
        }
    }
    let context_error = || {
        DeadgateError::new(format!(
            "protection export at {shown} carries a context that is not a non empty string"
        ))
    };
    let mut contexts: Vec<String> = Vec::new();
    match &checks {
        Some(JsonValue::List(entries)) if !entries.is_empty() => {
            for entry in entries {
                let context = entry.get("context");
                match context {
                    Some(JsonValue::Text(text)) if !text.is_empty() => contexts.push(text.clone()),
                    _ => return Err(context_error()),
                }
            }
        }
        _ => {
            if let Some(JsonValue::List(legacy)) = required.get("contexts") {
                for context in legacy {
                    match context {
                        JsonValue::Text(text) if !text.is_empty() => contexts.push(text.clone()),
                        _ => return Err(context_error()),
                    }
                }
            }
        }
    }
    if contexts.is_empty() {
        return Err(DeadgateError::new(format!(
            "protection export at {shown} carries no required status checks"
        )));
    }
    let mut seen: Vec<String> = Vec::new();
    for context in contexts {
        if !seen.contains(&context) {
            seen.push(context);
        }
    }
    Ok(seen)
}

fn trigger_block(data: &YamlValue) -> YamlValue {
    // The YAML parser hands the unquoted on key back as the
    // boolean true, and the string form is asked for first.
    if let Some(value) = data.get_text("on") {
        return value.clone();
    }
    data.get_key(&YamlKey::Bool(true))
        .cloned()
        .unwrap_or(YamlValue::Null)
}

/// A matrix value in its plain scalar form, for comparing and
/// rendering, the boolean and null spellings standing lowercase.
fn scalar_text(value: &YamlValue) -> String {
    match value {
        YamlValue::Bool(flag) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        YamlValue::Null => "null".to_string(),
        _ => python_str(value),
    }
}

type Combination = Vec<(YamlKey, YamlValue)>;

fn combination_get<'a>(combination: &'a Combination, key: &YamlKey) -> Option<&'a YamlValue> {
    combination
        .iter()
        .find(|(standing, _)| standing == key)
        .map(|(_, value)| value)
}

fn combination_update(combination: &mut Combination, entry: &Combination) {
    for (key, value) in entry {
        match combination.iter_mut().find(|(standing, _)| standing == key) {
            Some((_, standing)) => *standing = value.clone(),
            None => combination.push((key.clone(), value.clone())),
        }
    }
}

fn combination_render(combination: &Combination, axes: &[YamlKey]) -> String {
    let mut ordered: Vec<&YamlKey> = Vec::new();
    for axis in axes {
        if combination_get(combination, axis).is_some() {
            ordered.push(axis);
        }
    }
    for (key, _) in combination {
        if !axes.contains(key) {
            ordered.push(key);
        }
    }
    ordered
        .iter()
        .map(|key| scalar_text(combination_get(combination, key).expect("key stands")))
        .collect::<Vec<String>>()
        .join(", ")
}

/// The combination strings the job strategy matrix generates,
/// or None when the job carries no matrix.
fn matrix_combinations(spec: &YamlValue) -> Option<Vec<String>> {
    let strategy = spec.get_text("strategy")?;
    strategy.as_mapping()?;
    let matrix = strategy.get_text("matrix")?;
    let matrix_pairs = matrix.as_mapping()?;
    let axes: Vec<YamlKey> = matrix_pairs
        .iter()
        .map(|(key, _)| key.clone())
        .filter(|key| {
            *key != YamlKey::Text("include".to_string())
                && *key != YamlKey::Text("exclude".to_string())
        })
        .collect();
    let dict_entries = |key: &str| -> Vec<Combination> {
        match matrix.get_text(key) {
            Some(YamlValue::List(items)) => items
                .iter()
                .filter_map(|item| item.as_mapping().cloned())
                .collect(),
            _ => Vec::new(),
        }
    };
    let includes = dict_entries("include");
    let excludes = dict_entries("exclude");
    let mut combinations: Vec<Combination>;
    if axes.is_empty() {
        combinations = includes.clone();
    } else {
        let mut originals: Vec<Combination> = vec![Vec::new()];
        for axis in &axes {
            let axis_value = matrix.get_key(axis).cloned().unwrap_or(YamlValue::Null);
            let pool: Vec<YamlValue> = match &axis_value {
                YamlValue::List(items) => items.clone(),
                other => vec![other.clone()],
            };
            let mut next: Vec<Combination> = Vec::new();
            for prefix in &originals {
                for value in &pool {
                    let mut grown = prefix.clone();
                    grown.push((axis.clone(), value.clone()));
                    next.push(grown);
                }
            }
            originals = next;
        }
        combinations = originals.clone();
        for entry in &includes {
            let mut merged = false;
            for (position, original) in originals.iter().enumerate() {
                let overwrites =
                    entry
                        .iter()
                        .any(|(key, value)| match combination_get(original, key) {
                            Some(standing) => scalar_text(standing) != scalar_text(value),
                            None => false,
                        });
                if !overwrites {
                    combination_update(&mut combinations[position], entry);
                    merged = true;
                }
            }
            if !merged {
                combinations.push(entry.clone());
            }
        }
    }
    let mut kept = Vec::new();
    for combination in &combinations {
        let excluded = excludes.iter().any(|entry| {
            entry
                .iter()
                .all(|(key, value)| match combination_get(combination, key) {
                    Some(standing) => scalar_text(standing) == scalar_text(value),
                    None => false,
                })
        });
        if !excluded {
            kept.push(combination_render(combination, &axes));
        }
    }
    Some(kept)
}

/// The suffix of a file name as the Python path reading takes
/// it, the part from its last dot where that dot does not open
/// the name. The name is a run of bytes off the disk, and the
/// dot the reading looks for is a byte, so the reading runs on
/// the bytes.
fn python_suffix(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|byte| *byte == b'.') {
        Some(index) if index > 0 => &name[index..],
        _ => &[],
    }
}

/// The stem of a file name as the Python path reading takes
/// it, the name with its suffix cut.
fn python_stem(name: &[u8]) -> &[u8] {
    let suffix = python_suffix(name);
    &name[..name.len() - suffix.len()]
}

#[cfg(unix)]
fn os_string_from_bytes(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}

#[cfg(not(unix))]
fn os_string_from_bytes(bytes: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(unix)]
fn name_bytes(name: &OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    name.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn name_bytes(name: &OsString) -> Vec<u8> {
    name.to_string_lossy().into_owned().into_bytes()
}

/// The folder word as the runtime path reading normalises it.
/// Empty segments and dot segments fall away, a leading slash
/// stands, a doubled leading slash stands as the root, the
/// parent word stands as a segment, and a word with nothing
/// left stands as the dot.
fn normalised_folder(word: &PyText) -> PyText {
    let slash = u32::from('/');
    let points = word.points();
    let mut slashes = 0;
    while slashes < points.len() && points[slashes] == slash {
        slashes += 1;
    }
    let mut out = PyText::new();
    if slashes == 2 {
        out.push_str("//");
    } else if slashes >= 1 {
        out.push_str("/");
    }
    let mut first = true;
    for segment in points.split(|point| *point == slash) {
        if segment.is_empty() || segment == [u32::from('.')] {
            continue;
        }
        if !first {
            out.push_str("/");
        }
        first = false;
        out.push_points(segment);
    }
    if out.is_empty() {
        out.push_str(".");
    }
    out
}

/// The spelling of one member of a folder, the folder word and
/// the member name joined the way the runtime path reading
/// joins them.
fn member_text(folder: &PyText, name: &PyText) -> PyText {
    let mut out = PyText::new();
    if folder.equals_str(".") {
        out.push_text(name);
        return out;
    }
    out.push_text(folder);
    let points = folder.points();
    if points.last() != Some(&u32::from('/')) {
        out.push_str("/");
    }
    out.push_text(name);
    out
}

/// Every workflow definition in a folder, in filename order.
pub fn load_workflows(folder: &Path) -> Result<Vec<Workflow>, DeadgateError> {
    let normalised = normalised_folder(&PyText::from_os_str(folder.as_os_str()));
    let folder_shown = normalised.stderr_spelling();
    let folder_error =
        || DeadgateError::new(format!("workflows folder cannot be read at {folder_shown}"));
    let folder_path = PathBuf::from(os_string_from_bytes(normalised.stdout_bytes()));
    if !folder_path.is_dir() {
        return Err(folder_error());
    }
    let mut members: Vec<(PyText, PathBuf, Vec<u8>)> = Vec::new();
    let entries = std::fs::read_dir(&folder_path).map_err(|_| folder_error())?;
    for entry in entries {
        let entry = entry.map_err(|_| folder_error())?;
        let raw_name = entry.file_name();
        let bytes_of_name = name_bytes(&raw_name);
        let suffix = python_suffix(&bytes_of_name);
        if suffix != b".yml" && suffix != b".yaml" {
            continue;
        }
        let full = entry.path();
        let is_file = std::fs::metadata(&full)
            .map(|meta| meta.is_file())
            .unwrap_or(false);
        if is_file {
            let member = member_text(&normalised, &PyText::from_bytes(&bytes_of_name));
            members.push((member, full, bytes_of_name));
        }
    }
    members.sort_by(|a, b| a.0.cmp(&b.0));
    let mut workflows = Vec::new();
    for (member, full_path, bytes_of_name) in &members {
        let full = member.stderr_spelling();
        let bytes = std::fs::read(full_path)
            .map_err(|_| DeadgateError::new(format!("workflow file cannot be read at {full}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| DeadgateError::new(format!("workflow file at {full} does not parse")))?;
        let data = parse_yaml(&text)
            .ok_or_else(|| DeadgateError::new(format!("workflow file at {full} does not parse")))?;
        if !data.is_mapping() {
            return Err(DeadgateError::new(format!(
                "workflow file at {full} is not a mapping"
            )));
        }
        let jobs = data.get_text("jobs");
        let jobs_pairs = match jobs {
            Some(value) => match value.as_mapping() {
                Some(pairs) if !pairs.is_empty() => pairs.clone(),
                _ => {
                    return Err(DeadgateError::new(format!(
                        "workflow file at {full} carries no jobs"
                    )))
                }
            },
            None => {
                return Err(DeadgateError::new(format!(
                    "workflow file at {full} carries no jobs"
                )))
            }
        };
        let workflow_name = match data.get_text("name") {
            Some(YamlValue::Text(text)) if !text.is_empty() => PyText::from_plain(text),
            _ => PyText::from_bytes(python_stem(bytes_of_name)),
        };
        let mut job_map: Vec<(String, JobInfo)> = Vec::new();
        for (job_key, spec) in &jobs_pairs {
            let job_id_text = python_str(&key_to_value(job_key));
            let spec_map = if spec.is_mapping() {
                spec.clone()
            } else {
                YamlValue::Mapping(Vec::new())
            };
            let display = match spec_map.get_text("name") {
                Some(YamlValue::Text(text)) if !text.is_empty() => text.clone(),
                _ => job_id_text.clone(),
            };
            let has_if =
                matches!(spec_map.get_text("if"), Some(YamlValue::Text(text)) if !text.is_empty());
            let info = JobInfo {
                job: job_id_text.clone(),
                job_name: display,
                has_if,
                combinations: matrix_combinations(&spec_map),
            };
            match job_map
                .iter_mut()
                .find(|(standing, _)| *standing == job_id_text)
            {
                Some((_, standing)) => *standing = info,
                None => job_map.push((job_id_text, info)),
            }
        }
        workflows.push(Workflow {
            workflow: workflow_name,
            file: PyText::from_bytes(bytes_of_name),
            triggers: trigger_block(&data),
            jobs: job_map,
        });
    }
    Ok(workflows)
}

/// Read one timestamp field of a history record, a missing or
/// null field reading as no timestamp and any other value that
/// does not read standing as the refusal the caller names.
pub fn parse_timestamp_field(value: Option<&JsonValue>) -> Result<Option<i128>, DeadgateError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Text(text)) => match parse_timestamp_micros(text) {
            Some(instant) => Ok(Some(instant)),
            None => Err(DeadgateError::new("timestamp does not parse".to_string())),
        },
        Some(_) => Err(DeadgateError::new("timestamp is not a string".to_string())),
    }
}

/// The job records of an execution history export, in file order.
pub fn load_history(path: &Path) -> Result<Vec<Record>, DeadgateError> {
    let shown = path_text(path);
    let data = read_json_document(path, "history export")?;
    let jobs = match data.get("jobs") {
        Some(JsonValue::List(items)) => items.clone(),
        _ => {
            return Err(DeadgateError::new(format!(
                "history export at {shown} carries no jobs list"
            )))
        }
    };
    let mut records = Vec::new();
    for (position, entry) in jobs.iter().enumerate() {
        if !entry.is_object() {
            return Err(DeadgateError::new(format!(
                "history record {position} is not a mapping"
            )));
        }
        let name = match entry.get("name") {
            Some(JsonValue::Text(text)) if !text.is_empty() => text.clone(),
            _ => {
                return Err(DeadgateError::new(format!(
                    "history record {position} carries a name that is not a non empty string"
                )))
            }
        };
        let started = parse_timestamp_field(entry.get("started_at")).map_err(|_| {
            DeadgateError::new(format!(
                "history record {position} carries a timestamp that does not parse"
            ))
        })?;
        let completed = parse_timestamp_field(entry.get("completed_at")).map_err(|_| {
            DeadgateError::new(format!(
                "history record {position} carries a timestamp that does not parse"
            ))
        })?;
        let status = entry.get("status").cloned().unwrap_or(JsonValue::Null);
        if status.is_text("completed") && started.is_none() && completed.is_none() {
            return Err(DeadgateError::new(format!(
                "history record {position} is completed and carries neither a start nor a completion timestamp"
            )));
        }
        records.push(Record {
            name,
            workflow_name: entry
                .get("workflow_name")
                .cloned()
                .unwrap_or(JsonValue::Null),
            status,
            conclusion: entry.get("conclusion").cloned().unwrap_or(JsonValue::Null),
            started_at: started,
            completed_at: completed,
        });
    }
    Ok(records)
}

/// The first and last dates any record touches, as the window
/// object the report carries, or None when no record carries a
/// timestamp.
pub fn history_window(records: &[Record]) -> Option<JsonValue> {
    let mut stamps: Vec<i128> = Vec::new();
    for record in records {
        if let Some(stamp) = record.started_at {
            stamps.push(stamp);
        }
        if let Some(stamp) = record.completed_at {
            stamps.push(stamp);
        }
    }
    if stamps.is_empty() {
        return None;
    }
    let first = *stamps.iter().min().expect("stamps");
    let last = *stamps.iter().max().expect("stamps");
    Some(JsonValue::object(vec![
        ("first", JsonValue::text(date_text_of_instant(first))),
        ("last", JsonValue::text(date_text_of_instant(last))),
    ]))
}
