//! The closing suite of the port at the command line.
//!
//! The battery at tests/conformance/cli_cases.json stands
//! recorded from the Python command by the generator the round
//! keeps in its scratch record and is never edited by hand.
//! Every case runs the built binary in a fresh folder built
//! from the recorded setup, and the exit and both streams
//! compare byte for byte. A case whose recorded standard error
//! stands null is a usage refusal, and its standard error
//! compares against the usage line the shell itself prints,
//! taken from the binary and pinned here against the spelling
//! the Python parser prints. Cases in the band family carry
//! the port's stated answers under PORT2-4 where the Python
//! completes or dies, as the generator's record states.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use deadgate::jsonvalue::{parse_json, JsonValue};

/// The usage block the Python parser prints, two lines, as
/// the argument probes recorded it. The shell's own usage
/// line must spell exactly this.
const USAGE_LINE: &str = "usage: deadgate [-h] --protection PROTECTION --workflows \
WORKFLOWS --history\n                HISTORY [--branch BRANCH] [--markdown]\n";

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_deadgate")
}

fn temp_root(slug: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("deadgate-cli-{}-{slug}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clear the case folder");
    }
    std::fs::create_dir_all(&root).expect("make the case folder");
    root
}

fn base64_decode(text: &str) -> Vec<u8> {
    let mut values: Vec<u8> = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for ch in text.chars() {
        let digit = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            'a'..='z' => ch as u32 - 'a' as u32 + 26,
            '0'..='9' => ch as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            '=' => break,
            _ => continue,
        };
        buffer = (buffer << 6) | digit;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            values.push((buffer >> bits) as u8);
        }
    }
    values
}

fn field<'a>(value: &'a JsonValue, key: &str) -> &'a JsonValue {
    value
        .get(key)
        .unwrap_or_else(|| panic!("field {key} stands on the case"))
}

fn field_text<'a>(value: &'a JsonValue, key: &str) -> &'a str {
    field(value, key)
        .as_text()
        .unwrap_or_else(|| panic!("field {key} is text"))
}

fn substitute(data: &[u8], root: &Path) -> Vec<u8> {
    let needle = b"{DIR}";
    let replacement = root.to_string_lossy().into_owned().into_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < data.len() {
        if data[index..].starts_with(needle) {
            out.extend_from_slice(&replacement);
            index += needle.len();
        } else {
            out.push(data[index]);
            index += 1;
        }
    }
    out
}

#[cfg(unix)]
fn os_from_bytes(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}

#[cfg(not(unix))]
fn os_from_bytes(bytes: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&bytes).into_owned())
}

/// The usage line the shell itself prints, taken from a run
/// with no arguments, whose refusal is a usage refusal by
/// construction.
fn shell_usage() -> Vec<u8> {
    let output = Command::new(binary())
        .output()
        .expect("run the binary for its usage line");
    assert_eq!(output.status.code(), Some(2), "no arguments refuses");
    assert!(output.stdout.is_empty(), "a usage refusal prints no report");
    output.stderr
}

fn run_case(index: usize, case: &JsonValue, usage: &[u8]) {
    let name = field_text(case, "name");
    let root = temp_root(&format!("case{index:03}"));
    if let Some(JsonValue::List(dirs)) = case.get("dirs") {
        for dir in dirs {
            let rel = dir.as_text().expect("a dir entry is text");
            std::fs::create_dir_all(root.join(rel)).expect("make the recorded dir");
        }
    }
    if let Some(JsonValue::Object(files)) = case.get("files") {
        for (rel, entry) in files {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().expect("a file stands in a folder"))
                .expect("make the file folder");
            if let Some(text) = entry.get("text") {
                std::fs::write(&path, text.as_text().expect("file text"))
                    .expect("write the recorded file");
            } else {
                let bytes = base64_decode(field_text(entry, "b64"));
                std::fs::write(&path, bytes).expect("write the recorded file");
            }
        }
    }
    if let Some(JsonValue::Object(links)) = case.get("symlinks") {
        for (rel, target) in links {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().expect("a link stands in a folder"))
                .expect("make the link folder");
            #[cfg(unix)]
            std::os::unix::fs::symlink(target.as_text().expect("link target"), &path)
                .expect("make the recorded link");
        }
    }
    let mut argv: Vec<OsString> = Vec::new();
    if let Some(JsonValue::List(tokens)) = case.get("argv") {
        for token in tokens {
            if let Some(text) = token.get("text") {
                let text = text.as_text().expect("token text");
                argv.push(OsString::from(
                    text.replace("{DIR}", &root.to_string_lossy()),
                ));
            } else {
                let bytes = base64_decode(field_text(token, "b64"));
                argv.push(os_from_bytes(substitute(&bytes, &root)));
            }
        }
    }
    let output = Command::new(binary())
        .args(&argv)
        .current_dir(&root)
        .output()
        .expect("run the binary");
    let expect = field(case, "expect");
    let wanted_exit = match field(expect, "exit") {
        JsonValue::Int(value) => *value,
        other => panic!("case {name} records an exit, got {other:?}"),
    };
    assert_eq!(
        output.status.code(),
        Some(wanted_exit as i32),
        "case {name} exit"
    );
    let wanted_stdout = substitute(&base64_decode(field_text(expect, "stdout_b64")), &root);
    assert_eq!(output.stdout, wanted_stdout, "case {name} stdout");
    match expect.get("stderr_b64") {
        Some(JsonValue::Null) => {
            assert_eq!(output.stderr, usage, "case {name} usage stderr");
        }
        Some(JsonValue::Text(encoded)) => {
            let wanted = substitute(&base64_decode(encoded), &root);
            assert_eq!(output.stderr, wanted, "case {name} stderr");
        }
        other => panic!("case {name} records its stderr, got {other:?}"),
    }
}

#[test]
fn cli_battery_holds() {
    let usage = shell_usage();
    assert_eq!(
        usage,
        USAGE_LINE.as_bytes(),
        "the shell's usage line is the recorded spelling"
    );
    let document =
        parse_json(include_str!("conformance/cli_cases.json")).expect("the battery decodes");
    let cases = document
        .get("cases")
        .and_then(|value| value.as_list())
        .expect("the battery carries its cases")
        .clone();
    let mut families: Vec<(String, usize)> = Vec::new();
    for case in &cases {
        let family = field_text(case, "family").to_string();
        match families
            .iter_mut()
            .find(|(standing, _)| *standing == family)
        {
            Some((_, count)) => *count += 1,
            None => families.push((family, 1)),
        }
    }
    let census: Vec<(&str, usize)> = vec![
        ("baseline", 2),
        ("arguments", 45),
        ("refusals", 8),
        ("folders", 13),
        ("branch", 12),
        ("paths", 3),
        ("band", 5),
    ];
    assert_eq!(cases.len(), 88, "the battery census stands");
    for (family, count) in census {
        let standing = families
            .iter()
            .find(|(standing, _)| standing == family)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        assert_eq!(standing, count, "the {family} family census stands");
    }
    for (index, case) in cases.iter().enumerate() {
        run_case(index, case, &usage);
    }
}

/// The pin of PORT2-4. A value nested 4500 levels deep in an
/// ignored position of the protection document, and the same
/// in the history document, completes through the Python
/// command with the report unchanged, as the band probe
/// recorded, and the port refuses both with the standing not
/// valid JSON refusal and never produces that report. The
/// refusal is the discrimination, a case the Python completes
/// standing refused here.
#[test]
fn test_an_ignored_position_nested_past_the_port_bound_is_refused() {
    let root = temp_root("bandpin");
    let deep = format!("{}1{}", "[".repeat(4500), "]".repeat(4500));
    let workflows = root.join("workflows");
    std::fs::create_dir_all(&workflows).expect("make the workflows folder");
    std::fs::write(
        workflows.join("ci.yml"),
        "name: ci\non: push\njobs:\n  build:\n    steps: []\n",
    )
    .expect("write the workflow");
    let protection = root.join("protection.json");
    let history = root.join("history.json");
    let plain_protection = "{\"required_status_checks\": {\"checks\": \
        [{\"context\": \"ci / build\"}]}}";
    let deep_protection = format!(
        "{{\"required_status_checks\": {{\"checks\": \
         [{{\"context\": \"ci / build\"}}]}}, \"spare\": {deep}}}"
    );
    std::fs::write(&protection, &deep_protection).expect("write deep protection");
    std::fs::write(&history, "{\"jobs\": []}").expect("write history");
    let output = Command::new(binary())
        .args([
            OsString::from("--protection"),
            protection.clone().into_os_string(),
            OsString::from("--workflows"),
            workflows.clone().into_os_string(),
            OsString::from("--history"),
            history.clone().into_os_string(),
        ])
        .output()
        .expect("run the binary");
    assert_eq!(output.status.code(), Some(2), "the deep protection refuses");
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr text"),
        format!(
            "deadgate: protection export at {} is not valid JSON\n",
            protection.to_string_lossy()
        ),
        "the deep protection refusal wording"
    );
    std::fs::write(&protection, plain_protection).expect("write plain protection");
    let deep_history = format!("{{\"jobs\": [], \"spare\": {deep}}}");
    std::fs::write(&history, &deep_history).expect("write deep history");
    let output = Command::new(binary())
        .args([
            OsString::from("--protection"),
            protection.into_os_string(),
            OsString::from("--workflows"),
            workflows.into_os_string(),
            OsString::from("--history"),
            history.clone().into_os_string(),
        ])
        .output()
        .expect("run the binary");
    assert_eq!(output.status.code(), Some(2), "the deep history refuses");
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr text"),
        format!(
            "deadgate: history export at {} is not valid JSON\n",
            history.to_string_lossy()
        ),
        "the deep history refusal wording"
    );
}

#[test]
fn test_example_reports_regenerate_byte_identical() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let examples = root.join("examples");
    for markdown in [false, true] {
        let mut command = Command::new(binary());
        command.args([
            OsString::from("--protection"),
            examples.join("protection.json").into_os_string(),
            OsString::from("--workflows"),
            examples.join("workflows").into_os_string(),
            OsString::from("--history"),
            examples.join("history.json").into_os_string(),
        ]);
        if markdown {
            command.arg("--markdown");
        }
        let output = command.output().expect("run the binary");
        assert_eq!(output.status.code(), Some(0), "the examples complete");
        let committed =
            std::fs::read(examples.join(if markdown { "report.md" } else { "report.json" }))
                .expect("the committed example stands");
        assert_eq!(
            output.stdout,
            committed,
            "the {} example regenerates byte identical",
            if markdown { "markdown" } else { "json" }
        );
    }
}

#[test]
fn test_report_is_byte_identical_across_two_runs() {
    let root = temp_root("tworuns");
    std::fs::write(
        root.join("protection.json"),
        "{\"required_status_checks\": {\"checks\": [{\"context\": \"ci / build\"}]}}",
    )
    .expect("write protection");
    let workflows = root.join("workflows");
    std::fs::create_dir_all(&workflows).expect("make the workflows folder");
    std::fs::write(
        workflows.join("ci.yml"),
        "name: ci\non: push\njobs:\n  build:\n    steps: []\n",
    )
    .expect("write the workflow");
    std::fs::write(root.join("history.json"), "{\"jobs\": []}").expect("write history");
    let run = || {
        Command::new(binary())
            .args([
                OsString::from("--protection"),
                root.join("protection.json").into_os_string(),
                OsString::from("--workflows"),
                workflows.clone().into_os_string(),
                OsString::from("--history"),
                root.join("history.json").into_os_string(),
            ])
            .output()
            .expect("run the binary")
    };
    let first = run();
    let second = run();
    assert_eq!(first.status.code(), Some(0), "the first run completes");
    assert_eq!(second.status.code(), Some(0), "the second run completes");
    assert_eq!(first.stdout, second.stdout, "two runs print the same bytes");
    assert_eq!(first.stderr, second.stderr, "two runs refuse the same way");
}

/// The write standing of PORT2-5. The report outgrows the
/// pipe buffer, the reader closes its end, and the write
/// fails, so the port ends with its refusal line at exit two
/// where the Python command ends at exit 120.
#[test]
fn a_broken_stdout_pipe_reads_as_the_write_refusal() {
    let root = temp_root("brokenpipe");
    let mut contexts = String::new();
    for index in 0..3000 {
        if index > 0 {
            contexts.push_str(", ");
        }
        contexts.push_str(&format!("{{\"context\": \"check {index}\"}}"));
    }
    std::fs::write(
        root.join("protection.json"),
        format!("{{\"required_status_checks\": {{\"checks\": [{contexts}]}}}}"),
    )
    .expect("write protection");
    std::fs::create_dir_all(root.join("workflows")).expect("make the workflows folder");
    std::fs::write(root.join("history.json"), "{\"jobs\": []}").expect("write history");
    let mut child = Command::new(binary())
        .args([
            OsString::from("--protection"),
            root.join("protection.json").into_os_string(),
            OsString::from("--workflows"),
            root.join("workflows").into_os_string(),
            OsString::from("--history"),
            root.join("history.json").into_os_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the binary");
    // Close the read end of the report pipe while the report
    // is still being written.
    drop(child.stdout.take());
    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .expect("stderr stands")
        .read_to_end(&mut stderr)
        .expect("read stderr");
    let status = child.wait().expect("the binary ends");
    assert_eq!(status.code(), Some(2), "the failed write exits two");
    assert_eq!(
        stderr,
        b"deadgate: the report cannot be written\n".to_vec(),
        "the failed write reads as the PORT2-5 refusal"
    );
}
