//! The deadgate command, the binary door of the crate. Its
//! flags are the flags of the Python command, its refusals are
//! the deadgate errors of the library printed the way the
//! Python command prints them with an exit code of two, and
//! its usage text is its own.

use std::process::ExitCode;

use deadgate::ingest::{history_window, load_history, load_protection, load_workflows};
use deadgate::report::{build_report, render_json, render_markdown};

const USAGE: &str = "usage: deadgate --protection <file> --workflows <folder> \
--history <file> [--branch <name>] [--markdown]";

fn run() -> Result<String, String> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let mut protection: Option<String> = None;
    let mut workflows: Option<String> = None;
    let mut history: Option<String> = None;
    let mut branch = "main".to_string();
    let mut markdown = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--protection" | "--workflows" | "--history" | "--branch" => {
                let value = args.get(index + 1).ok_or_else(|| USAGE.to_string())?;
                match arg.as_str() {
                    "--protection" => protection = Some(value.clone()),
                    "--workflows" => workflows = Some(value.clone()),
                    "--history" => history = Some(value.clone()),
                    _ => branch = value.clone(),
                }
                index += 2;
            }
            "--markdown" => {
                markdown = true;
                index += 1;
            }
            "--help" | "-h" => {
                return Err(USAGE.to_string());
            }
            _ => return Err(USAGE.to_string()),
        }
    }
    let (protection, workflows, history) = match (protection, workflows, history) {
        (Some(protection), Some(workflows), Some(history)) => (protection, workflows, history),
        _ => return Err(USAGE.to_string()),
    };
    let contexts = load_protection(&protection).map_err(|error| error.message)?;
    let workflow_list = load_workflows(&workflows).map_err(|error| error.message)?;
    let records = load_history(&history).map_err(|error| error.message)?;
    let window = history_window(&records);
    let report = build_report(&contexts, &workflow_list, &records, &branch, window)
        .map_err(|error| error.message)?;
    Ok(if markdown {
        render_markdown(&report)
    } else {
        render_json(&report)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(rendered) => {
            print!("{rendered}");
            ExitCode::from(0)
        }
        Err(message) => {
            if message == USAGE {
                eprintln!("{USAGE}");
            } else {
                eprintln!("deadgate: {message}");
            }
            ExitCode::from(2)
        }
    }
}
