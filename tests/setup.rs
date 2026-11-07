//! The setup obligation pins of the repo, carried over from
//! the retired Python suite under their own names, the README
//! count pin and the annex gate sentence pin among them as
//! PORT2-6 states. The gate sentence the CI pin names is the
//! gate of this slice, cargo test alone, the move the annex
//! sentence makes in this same commit.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|_| panic!("the repo file {relative} stands"))
}

#[test]
fn test_seed_files_present_and_licence_intact() {
    for name in ["README.md", "LICENSE", ".gitignore"] {
        assert!(
            repo_root().join(name).is_file(),
            "the seed file {name} stands"
        );
    }
    let licence = read_repo_file("LICENSE");
    assert!(licence.starts_with("MIT License"), "the licence stands");
    assert!(
        licence.contains("Permission is hereby granted"),
        "the licence grant stands"
    );
}

#[test]
fn test_readme_status_section_states_the_facts() {
    let readme = read_repo_file("README.md");
    assert!(readme.contains("## Status"), "the status section stands");
    let status = readme
        .split("## Status")
        .nth(1)
        .expect("the status section stands")
        .split("## Licence")
        .next()
        .expect("the status section ends at the licence");
    let lowered = status.to_lowercase();
    for fact in [
        "slice 2 is built",
        "contract.md",
        "deadgate-report/1",
        "agents.md",
        "canonical template",
        "pinned formats",
    ] {
        assert!(lowered.contains(fact), "the status states {fact}");
    }
    assert!(
        lowered.contains("closed rounds stand at 3 of 100"),
        "the status states the closed round count"
    );
}

#[test]
fn test_ci_runs_the_local_gate_command() {
    let ci = read_repo_file(".github/workflows/ci.yml");
    assert!(ci.contains("\n  push:"), "the push trigger stands");
    assert!(
        ci.contains("\n  pull_request:"),
        "the pull request trigger stands"
    );
    assert!(
        ci.contains("cargo test"),
        "the workflow runs the local gate command"
    );
    assert!(
        !ci.contains("python -m pytest"),
        "the workflow carries no Python job"
    );
}

#[test]
fn test_agents_md_stands_from_the_canonical_template() {
    let agents = read_repo_file("AGENTS.md");
    let marker = "<!-- repo annex below this line -->";
    assert!(agents.contains(marker), "the annex marker stands");
    assert!(agents.contains("## Repo annex"), "the annex stands");
    let annex = agents
        .split(marker)
        .nth(1)
        .expect("the annex stands below the marker");
    assert!(
        annex.contains("Deadgate asks whether it ran"),
        "the annex states the boundary"
    );
    assert!(
        annex.contains("The local gate is cargo test."),
        "the annex gate sentence stands at cargo test alone"
    );
}
