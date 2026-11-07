//! The reconciliation, mirroring the Python reconciliation
//! module obligation by obligation. Branch filter patterns are
//! read through the same translation into matching parts the
//! Python reading makes, a bracketed range that runs backwards
//! standing as the refusal the port contract states where the
//! Python command dies on it.

use std::collections::HashSet;

use crate::dates::date_text_of_instant;
use crate::errors::DeadgateError;
use crate::ingest::{JobInfo, Record, Workflow};
use crate::jsonvalue::JsonValue;
use crate::pytext::PyText;
use crate::yaml::python_str;
use crate::yamlvalue::{key_to_value, YamlValue};

pub const NOTE_PATH_FILTERED: &str = "path filtered trigger";
pub const NOTE_IF_CONDITION: &str = "job carries an if condition";

const RAN_CONCLUSIONS: [&str; 5] = ["success", "failure", "cancelled", "timed_out", "neutral"];
const BRANCH_EVENTS: [&str; 2] = ["push", "pull_request"];

/// One producer of a required context, as the report carries it.
#[derive(Debug, Clone)]
pub struct Producer {
    pub workflow: PyText,
    pub file: PyText,
    pub job: String,
    pub job_name: String,
}

/// One reconciled entry per required context.
#[derive(Debug, Clone)]
pub struct CheckEntry {
    pub context: String,
    pub status: String,
    pub producers: Vec<Producer>,
    pub last_run_date: Option<String>,
    pub notes: Vec<String>,
}

struct ProducerFlags {
    workflow: PyText,
    file: PyText,
    job: String,
    job_name: String,
    filtered_only: bool,
    has_if: bool,
}

// ----------------------------------------------------------------
// Branch filter patterns
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum AtomKind {
    Lit(char),
    Class(Vec<(char, char)>),
    StarSlash,
    StarAny,
}

#[derive(Debug, Clone, PartialEq)]
enum Quant {
    One,
    Question,
    Plus,
    Star,
    StarLazy,
    StarPossessive,
}

#[derive(Debug, Clone)]
struct Atom {
    kind: AtomKind,
    quant: Quant,
}

/// Compile one filter pattern into its matching atoms, under
/// the translation the Python reading makes. A bracketed range
/// that runs backwards refuses, naming the pattern.
fn compile_pattern(pattern: &str) -> Result<Vec<Atom>, DeadgateError> {
    let invalid = || {
        DeadgateError::new(format!(
            "branch pattern {pattern} is not a valid filter pattern"
        ))
    };
    let chars: Vec<char> = pattern.chars().collect();
    let mut atoms: Vec<Atom> = Vec::new();
    let mut quantified = false;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\\' && index + 1 < chars.len() {
            atoms.push(Atom {
                kind: AtomKind::Lit(chars[index + 1]),
                quant: Quant::One,
            });
            quantified = false;
            index += 2;
        } else if ch == '*' {
            if index + 1 < chars.len() && chars[index + 1] == '*' {
                atoms.push(Atom {
                    kind: AtomKind::StarAny,
                    quant: Quant::Star,
                });
                index += 2;
            } else {
                atoms.push(Atom {
                    kind: AtomKind::StarSlash,
                    quant: Quant::Star,
                });
                index += 1;
            }
            quantified = false;
        } else if ch == '?' || ch == '+' {
            if !atoms.is_empty() && !quantified {
                let last = atoms.last_mut().expect("atom");
                last.quant = match (&last.kind, ch) {
                    (AtomKind::StarSlash, '?') | (AtomKind::StarAny, '?') => Quant::StarLazy,
                    (AtomKind::StarSlash, '+') | (AtomKind::StarAny, '+') => Quant::StarPossessive,
                    (_, '?') => Quant::Question,
                    (_, '+') => Quant::Plus,
                    _ => unreachable!(),
                };
                quantified = true;
            } else {
                atoms.push(Atom {
                    kind: AtomKind::Lit(ch),
                    quant: Quant::One,
                });
                quantified = false;
            }
            index += 1;
        } else if ch == '[' {
            let closing = chars[index + 1..]
                .iter()
                .position(|standing| *standing == ']')
                .map(|offset| index + 1 + offset);
            match closing {
                Some(close) if close > index + 1 => {
                    let inner = &chars[index + 1..close];
                    let mut ranges: Vec<(char, char)> = Vec::new();
                    let mut item = 0;
                    while item < inner.len() {
                        let start = inner[item];
                        if item + 1 < inner.len()
                            && inner[item + 1] == '-'
                            && item + 2 < inner.len()
                        {
                            let end = inner[item + 2];
                            if end < start {
                                return Err(invalid());
                            }
                            ranges.push((start, end));
                            item += 3;
                        } else {
                            ranges.push((start, start));
                            item += 1;
                        }
                    }
                    atoms.push(Atom {
                        kind: AtomKind::Class(ranges),
                        quant: Quant::One,
                    });
                    quantified = false;
                    index = close + 1;
                }
                _ => {
                    atoms.push(Atom {
                        kind: AtomKind::Lit('['),
                        quant: Quant::One,
                    });
                    quantified = false;
                    index += 1;
                }
            }
        } else {
            atoms.push(Atom {
                kind: AtomKind::Lit(ch),
                quant: Quant::One,
            });
            quantified = false;
            index += 1;
        }
    }
    Ok(atoms)
}

/// Whether one atom takes one point of the name. The name
/// stands in points, because the branch word off the command
/// line can carry a point no character stands for, and the
/// pattern atoms compare against the point itself.
fn atom_matches(kind: &AtomKind, point: u32) -> bool {
    match kind {
        AtomKind::Lit(expected) => u32::from(*expected) == point,
        AtomKind::Class(ranges) => ranges
            .iter()
            .any(|(low, high)| u32::from(*low) <= point && point <= u32::from(*high)),
        AtomKind::StarSlash => point != u32::from('/'),
        // The double star translates to the dot of the pattern
        // engine, and the dot never matches a line break, while
        // the single star class is a negated class and does.
        AtomKind::StarAny => point != u32::from('\n'),
    }
}

fn star_predicate(kind: &AtomKind, point: u32) -> bool {
    match kind {
        AtomKind::StarSlash => point != u32::from('/'),
        AtomKind::StarAny => point != u32::from('\n'),
        _ => true,
    }
}

fn match_from(
    atoms: &[Atom],
    points: &[u32],
    atom_index: usize,
    char_index: usize,
    seen: &mut HashSet<(usize, usize)>,
) -> bool {
    if atom_index == atoms.len() {
        return char_index == points.len();
    }
    if !seen.insert((atom_index, char_index)) {
        return false;
    }
    let atom = &atoms[atom_index];
    match atom.quant {
        Quant::One => {
            char_index < points.len()
                && atom_matches(&atom.kind, points[char_index])
                && match_from(atoms, points, atom_index + 1, char_index + 1, seen)
        }
        Quant::Question => {
            (char_index < points.len()
                && atom_matches(&atom.kind, points[char_index])
                && match_from(atoms, points, atom_index + 1, char_index + 1, seen))
                || match_from(atoms, points, atom_index + 1, char_index, seen)
        }
        Quant::Plus => {
            char_index < points.len()
                && atom_matches(&atom.kind, points[char_index])
                && (match_from(atoms, points, atom_index, char_index + 1, seen)
                    || match_from(atoms, points, atom_index + 1, char_index + 1, seen))
        }
        Quant::Star | Quant::StarLazy => {
            let mut end = char_index;
            loop {
                if match_from(atoms, points, atom_index + 1, end, seen) {
                    return true;
                }
                if end < points.len() && star_predicate(&atom.kind, points[end]) {
                    end += 1;
                } else {
                    return false;
                }
            }
        }
        Quant::StarPossessive => {
            let mut end = char_index;
            while end < points.len() && star_predicate(&atom.kind, points[end]) {
                end += 1;
            }
            match_from(atoms, points, atom_index + 1, end, seen)
        }
    }
}

fn pattern_matches(pattern: &str, name: &[u32]) -> Result<bool, DeadgateError> {
    let atoms = compile_pattern(pattern)?;
    let mut seen = HashSet::new();
    Ok(match_from(&atoms, name, 0, 0, &mut seen))
}

/// Whether a pattern list claims a name, its patterns applied
/// in the order the file writes them so the last matching
/// pattern decides.
fn pattern_list_claims(patterns: &[String], name: &[u32]) -> Result<bool, DeadgateError> {
    let mut claimed = false;
    for pattern in patterns {
        if let Some(stripped) = pattern.strip_prefix('!') {
            if pattern_matches(stripped, name)? {
                claimed = false;
            }
        } else if pattern_matches(pattern, name)? {
            claimed = true;
        }
    }
    Ok(claimed)
}

/// The patterns one branch filter key carries, or None when
/// the key stands absent or null.
fn filter_patterns(config: &YamlValue, key: &str) -> Option<Vec<String>> {
    let value = config.get_text(key)?;
    match value {
        YamlValue::Null => None,
        YamlValue::List(items) => Some(items.iter().map(python_str).collect()),
        other => Some(vec![python_str(other)]),
    }
}

fn branch_admitted(config: &YamlValue, branch: &PyText) -> Result<bool, DeadgateError> {
    if let Some(branches) = filter_patterns(config, "branches") {
        if !pattern_list_claims(&branches, branch.points())? {
            return Ok(false);
        }
    }
    if let Some(ignore) = filter_patterns(config, "branches-ignore") {
        if pattern_list_claims(&ignore, branch.points())? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Trigger events mapped to their config values, from any
/// trigger form, the event names in their string form.
fn event_configs(triggers: &YamlValue) -> Vec<(String, YamlValue)> {
    let empty = || YamlValue::Mapping(Vec::new());
    let mut configs: Vec<(String, YamlValue)> = Vec::new();
    let mut insert = |name: String, config: YamlValue| match configs
        .iter_mut()
        .find(|(standing, _)| *standing == name)
    {
        Some((_, standing)) => *standing = config,
        None => configs.push((name, config)),
    };
    match triggers {
        YamlValue::Text(text) => insert(text.clone(), empty()),
        YamlValue::List(items) => {
            for item in items {
                insert(python_str(item), empty());
            }
        }
        YamlValue::Mapping(pairs) => {
            for (key, config) in pairs {
                let config = if config.is_mapping() {
                    config.clone()
                } else {
                    empty()
                };
                insert(python_str(&key_to_value(key)), config);
            }
        }
        _ => {}
    }
    configs
}

/// Whether the workflow can fire for the branch, and whether
/// it fires for the branch only through path filtered events.
pub fn workflow_fires(workflow: &Workflow, branch: &PyText) -> Result<(bool, bool), DeadgateError> {
    let configs = event_configs(&workflow.triggers);
    let mut fires = false;
    let mut unfiltered = false;
    for event in BRANCH_EVENTS {
        let Some((_, config)) = configs.iter().find(|(name, _)| name == event) else {
            continue;
        };
        if !branch_admitted(config, branch)? {
            continue;
        }
        fires = true;
        if config.get_text("paths").is_none() && config.get_text("paths-ignore").is_none() {
            unfiltered = true;
        }
    }
    Ok((fires, fires && !unfiltered))
}

/// The check names one job produces, as bare and workflow form
/// pairs, one pair per matrix combination. The forms stand in
/// points because the workflow name they build on can carry a
/// point off the file name bytes.
fn job_forms(workflow_name: &PyText, job: &JobInfo) -> Vec<(PyText, PyText)> {
    let display = PyText::from_plain(&job.job_name);
    let form = |bare: PyText| {
        let mut full = workflow_name.clone();
        full.push_str(" / ");
        full.push_text(&bare);
        (bare, full)
    };
    match &job.combinations {
        None => vec![form(display)],
        Some(combinations) => combinations
            .iter()
            .map(|suffix| {
                let mut bare = display.clone();
                bare.push_str(" (");
                bare.push_str(suffix);
                bare.push_str(")");
                form(bare)
            })
            .collect(),
    }
}

fn record_ran(record: &Record) -> bool {
    record.status.is_text("completed")
        && RAN_CONCLUSIONS
            .iter()
            .any(|word| record.conclusion.is_text(word))
}

fn record_skipped(record: &Record) -> bool {
    record.status.is_text("completed") && record.conclusion.is_text("skipped")
}

fn last_run_date(ran_records: &[&Record]) -> Option<String> {
    let mut finishing: Vec<i128> = Vec::new();
    for record in ran_records {
        let stamp = record.completed_at.or(record.started_at);
        if let Some(stamp) = stamp {
            finishing.push(stamp);
        }
    }
    if finishing.is_empty() {
        return None;
    }
    let latest = *finishing.iter().max().expect("stamps");
    Some(date_text_of_instant(latest))
}

/// One reconciled entry per required context, in protection
/// order.
pub fn reconcile(
    contexts: &[String],
    workflows: &[Workflow],
    records: &[Record],
    branch: &PyText,
) -> Result<Vec<CheckEntry>, DeadgateError> {
    let mut firing: Vec<(&Workflow, bool)> = Vec::new();
    for workflow in workflows {
        let (fires, filtered_only) = workflow_fires(workflow, branch)?;
        if fires {
            firing.push((workflow, filtered_only));
        }
    }
    let mut entries = Vec::new();
    for context in contexts {
        let mut producers: Vec<ProducerFlags> = Vec::new();
        let mut vouched: Vec<(PyText, Vec<PyText>)> = Vec::new();
        for (workflow, filtered_only) in &firing {
            let mut job_keys: Vec<&String> = workflow.jobs.iter().map(|(key, _)| key).collect();
            job_keys.sort();
            for job_key in job_keys {
                let job = &workflow
                    .jobs
                    .iter()
                    .find(|(key, _)| key == job_key)
                    .expect("job")
                    .1;
                for (bare, full) in job_forms(&workflow.workflow, job) {
                    if bare.equals_str(context) || full.equals_str(context) {
                        producers.push(ProducerFlags {
                            workflow: workflow.workflow.clone(),
                            file: workflow.file.clone(),
                            job: job.job.clone(),
                            job_name: job.job_name.clone(),
                            filtered_only: *filtered_only,
                            has_if: job.has_if,
                        });
                        vouched.push((workflow.workflow.clone(), vec![bare, full]));
                        break;
                    }
                }
            }
        }
        producers.sort_by(|a, b| (&a.file, &a.job).cmp(&(&b.file, &b.job)));
        let matched: Vec<&Record> = records
            .iter()
            .filter(|record| {
                vouched.iter().any(|(workflow_name, names)| {
                    let workflow_matches = match &record.workflow_name {
                        JsonValue::Text(text) => workflow_name.equals_str(text),
                        _ => false,
                    };
                    workflow_matches && names.iter().any(|name| name.equals_str(&record.name))
                })
            })
            .collect();
        let ran: Vec<&Record> = matched
            .iter()
            .filter(|record| record_ran(record))
            .cloned()
            .collect();
        let skipped: Vec<&Record> = matched
            .iter()
            .filter(|record| record_skipped(record))
            .cloned()
            .collect();
        let status = if producers.is_empty() {
            "no_producer"
        } else if !ran.is_empty() {
            "alive"
        } else if !skipped.is_empty() {
            "skipped_only"
        } else {
            "never_run"
        };
        let last = if !producers.is_empty() {
            last_run_date(&ran)
        } else {
            let tail = match context.rfind(" / ") {
                Some(index) => &context[index + 3..],
                None => context.as_str(),
            };
            let loose: Vec<&Record> = records
                .iter()
                .filter(|record| {
                    (record.name == *context || record.name == tail) && record_ran(record)
                })
                .collect();
            last_run_date(&loose)
        };
        let mut notes = Vec::new();
        if status == "never_run"
            && !producers.is_empty()
            && producers.iter().all(|producer| producer.filtered_only)
        {
            notes.push(NOTE_PATH_FILTERED.to_string());
        }
        if producers.iter().any(|producer| producer.has_if) {
            notes.push(NOTE_IF_CONDITION.to_string());
        }
        entries.push(CheckEntry {
            context: context.clone(),
            status: status.to_string(),
            producers: producers
                .into_iter()
                .map(|producer| Producer {
                    workflow: producer.workflow,
                    file: producer.file,
                    job: producer.job,
                    job_name: producer.job_name,
                })
                .collect(),
            last_run_date: last,
            notes,
        });
    }
    Ok(entries)
}
