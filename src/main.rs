//! The deadgate command, the binary door of the crate, rebuilt
//! as the Python command line whole. Its help and usage texts
//! stand as one literal, the spelling the Python parser
//! prints. Argument words arrive as operating system words and
//! never pass through a text conversion, so a path is carried
//! as a path and the branch word is carried in points. A
//! refusal from the library goes to standard error with the
//! deadgate prefix at exit two, an argument refusal prints the
//! usage text alone to standard error at exit two, and success
//! prints the report to standard output at exit zero. A failed
//! write of the report ends the command with the write refusal
//! of PORT2-5 at exit two.
//!
//! The argument reading follows the Python parser as the
//! probes in the round record it. A long word resolves by
//! exact match or by unique prefix, and a prefix matching two
//! long words refuses on the spot. Help fires the moment its
//! word is read, in a long word of its own or opening a short
//! cluster, ahead of any standing the scan has already taken
//! and behind any refusal the scan has already fired. A help
//! word carrying a value refuses, as does the markdown word
//! carrying one. An unknown word, a stray positional, and the
//! separator word stand aside as extras and refuse at the end
//! of the scan if the scan otherwise completes. A valued word
//! missing its value refuses on the spot, and a word counts as
//! a value when it does not open with a dash, or stands as the
//! lone dash, or reads as a negative number.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadgate::ingest::{history_window, load_history, load_protection, load_workflows};
use deadgate::pytext::PyText;
use deadgate::report::{build_report, render_json, render_markdown};

const USAGE: &str = "usage: deadgate [-h] --protection PROTECTION --workflows WORKFLOWS \
--history\n                HISTORY [--branch BRANCH] [--markdown]";

const HELP_TAIL: &str = "\n\nReconcile required status checks against the jobs CI actually \
executes and the\nhistory it left behind.\n\noptions:\n  -h, --help            show this help \
message and exit\n  --protection PROTECTION\n                        branch protection export, \
JSON\n  --workflows WORKFLOWS\n                        folder of workflow definitions\n  \
--history HISTORY     execution history export, JSON\n  --branch BRANCH       branch the \
protection export covers\n  --markdown            print the markdown rendering instead of the \
JSON\n                        rendering\n";

const WRITE_REFUSAL: &str = "deadgate: the report cannot be written";

#[derive(Clone, Copy, PartialEq, Eq)]
enum LongOption {
    Help,
    Protection,
    Workflows,
    History,
    Branch,
    Markdown,
}

const LONG_OPTIONS: [(&[u8], LongOption); 6] = [
    (b"help", LongOption::Help),
    (b"protection", LongOption::Protection),
    (b"workflows", LongOption::Workflows),
    (b"history", LongOption::History),
    (b"branch", LongOption::Branch),
    (b"markdown", LongOption::Markdown),
];

enum Outcome {
    Help,
    UsageRefusal,
    Run {
        protection: PathBuf,
        workflows: PathBuf,
        history: PathBuf,
        branch: PyText,
        markdown: bool,
    },
}

/// Resolve one long word name, the part before any value
/// separator. An exact match stands first, then a unique
/// prefix. The outer None stands for no match, and the error
/// stands for a prefix matching more than one long word.
fn resolve_long(name: &[u8]) -> Result<Option<LongOption>, ()> {
    for (full, option) in LONG_OPTIONS {
        if full == name {
            return Ok(Some(option));
        }
    }
    let mut candidates = LONG_OPTIONS
        .iter()
        .filter(|(full, _)| full.starts_with(name))
        .map(|(_, option)| *option);
    match (candidates.next(), candidates.next()) {
        (None, _) => Ok(None),
        (Some(option), None) => Ok(Some(option)),
        (Some(_), Some(_)) => Err(()),
    }
}

/// Whether a word reads as a negative number, a dash and then
/// digits with an optional fraction, or a dash and a dot and
/// then digits, the whole word and nothing else.
fn is_negative_number(bytes: &[u8]) -> bool {
    if bytes.first() != Some(&b'-') {
        return false;
    }
    let rest = &bytes[1..];
    let digits = |run: &[u8]| !run.is_empty() && run.iter().all(|byte| byte.is_ascii_digit());
    if let Some(after_dot) = rest.strip_prefix(b".") {
        return digits(after_dot);
    }
    let whole: usize = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if whole == 0 {
        return false;
    }
    if whole == rest.len() {
        return true;
    }
    if rest[whole] == b'.' {
        return digits(&rest[whole + 1..]);
    }
    false
}

/// Whether a word standing after a valued word counts as its
/// value, as the Python parser decides it.
fn is_value_like(bytes: &[u8]) -> bool {
    match bytes.first() {
        None => true,
        Some(b'-') => bytes.len() == 1 || is_negative_number(bytes),
        Some(_) => true,
    }
}

#[cfg(unix)]
fn word_bytes(word: &OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    word.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn word_bytes(word: &OsString) -> Vec<u8> {
    word.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

fn parse(words: Vec<OsString>) -> Outcome {
    let bytes: Vec<Vec<u8>> = words.iter().map(word_bytes).collect();
    let mut protection: Option<PathBuf> = None;
    let mut workflows: Option<PathBuf> = None;
    let mut history: Option<PathBuf> = None;
    let mut branch: Option<PyText> = None;
    let mut markdown = false;
    let mut extras = false;
    let mut after_separator = false;
    let mut index = 0;
    while index < words.len() {
        let word = &bytes[index];
        if after_separator {
            extras = true;
            index += 1;
            continue;
        }
        if word == b"--" {
            after_separator = true;
            extras = true;
            index += 1;
            continue;
        }
        if word.len() > 2 && word.starts_with(b"--") {
            let body = &word[2..];
            let (name, explicit) = match body.iter().position(|byte| *byte == b'=') {
                Some(position) => (&body[..position], Some(&body[position + 1..])),
                None => (body, None),
            };
            let option = match resolve_long(name) {
                Ok(Some(option)) => option,
                // An unknown long word stands aside as an extra.
                Ok(None) => {
                    extras = true;
                    index += 1;
                    continue;
                }
                // An ambiguous prefix refuses on the spot.
                Err(()) => return Outcome::UsageRefusal,
            };
            match option {
                LongOption::Help => {
                    if explicit.is_some() {
                        return Outcome::UsageRefusal;
                    }
                    return Outcome::Help;
                }
                LongOption::Markdown => {
                    if explicit.is_some() {
                        return Outcome::UsageRefusal;
                    }
                    markdown = true;
                    index += 1;
                }
                LongOption::Protection
                | LongOption::Workflows
                | LongOption::History
                | LongOption::Branch => {
                    let (value_word, value_bytes, step) = match explicit {
                        Some(value) => (None, value.to_vec(), 1),
                        None => match bytes.get(index + 1) {
                            Some(next) if is_value_like(next) => {
                                (Some(&words[index + 1]), next.clone(), 2)
                            }
                            _ => return Outcome::UsageRefusal,
                        },
                    };
                    if option == LongOption::Branch {
                        branch = Some(PyText::from_bytes(&value_bytes));
                    } else {
                        let path = match value_word {
                            Some(word) => PathBuf::from(word.clone()),
                            None => path_from_bytes(&value_bytes),
                        };
                        match option {
                            LongOption::Protection => protection = Some(path),
                            LongOption::Workflows => workflows = Some(path),
                            _ => history = Some(path),
                        }
                    }
                    index += step;
                }
            }
            continue;
        }
        if word.len() > 1 && word[0] == b'-' && !is_negative_number(word) {
            // A short cluster. Help fires when the cluster
            // opens with its letter, and a valued spelling of
            // that letter refuses. Any other cluster stands
            // aside as an extra, whole.
            if word[1] == b'h' {
                if word.len() > 2 && word[2] == b'=' {
                    return Outcome::UsageRefusal;
                }
                return Outcome::Help;
            }
            extras = true;
            index += 1;
            continue;
        }
        // A plain word, the empty word, the lone dash, and a
        // negative number standing alone are positional extras.
        extras = true;
        index += 1;
    }
    if extras {
        return Outcome::UsageRefusal;
    }
    match (protection, workflows, history) {
        (Some(protection), Some(workflows), Some(history)) => Outcome::Run {
            protection,
            workflows,
            history,
            branch: branch.unwrap_or_else(|| PyText::from_plain("main")),
            markdown,
        },
        _ => Outcome::UsageRefusal,
    }
}

fn run(
    protection: &Path,
    workflows: &Path,
    history: &Path,
    branch: &PyText,
    markdown: bool,
) -> Result<Vec<u8>, String> {
    let contexts = load_protection(protection).map_err(|error| error.message)?;
    let workflow_list = load_workflows(workflows).map_err(|error| error.message)?;
    let records = load_history(history).map_err(|error| error.message)?;
    let window = history_window(&records);
    let report = build_report(&contexts, &workflow_list, &records, branch, window)
        .map_err(|error| error.message)?;
    Ok(if markdown {
        render_markdown(&report)
    } else {
        render_json(&report).into_bytes()
    })
}

fn write_stdout(bytes: &[u8]) -> bool {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(bytes).is_ok() && lock.flush().is_ok()
}

fn write_stderr(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = lock.write_all(bytes);
    let _ = lock.flush();
}

fn main() -> ExitCode {
    let words: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse(words) {
        Outcome::Help => {
            let mut help = String::from(USAGE);
            help.push_str(HELP_TAIL);
            if write_stdout(help.as_bytes()) {
                ExitCode::from(0)
            } else {
                write_stderr(format!("{WRITE_REFUSAL}\n").as_bytes());
                ExitCode::from(2)
            }
        }
        Outcome::UsageRefusal => {
            write_stderr(format!("{USAGE}\n").as_bytes());
            ExitCode::from(2)
        }
        Outcome::Run {
            protection,
            workflows,
            history,
            branch,
            markdown,
        } => match run(&protection, &workflows, &history, &branch, markdown) {
            Ok(rendered) => {
                if write_stdout(&rendered) {
                    ExitCode::from(0)
                } else {
                    write_stderr(format!("{WRITE_REFUSAL}\n").as_bytes());
                    ExitCode::from(2)
                }
            }
            Err(message) => {
                write_stderr(format!("deadgate: {message}\n").as_bytes());
                ExitCode::from(2)
            }
        },
    }
}
