//! Tracker 3 for real, never the person's index: localsearch's own test sandbox (`localsearch
//! test-sandbox`) starts a private session bus with a miner that indexes one scratch folder
//! into a temporary store, and runs the `files_probe` example inside it, which asks
//! `TrackerIndex` through the provider exactly as the runner does. Checks the SPARQL, the pipe
//! and the cursor format end to end.
//!
//! Run explicitly: `cargo test -p sill-services --test it files_tracker_live -- --ignored
//! --nocapture` (needs localsearch; the folder is under the target directory because the miner
//! skips a tmpfs).

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "needs localsearch"]
fn tracker_finds_files_and_folders_by_name_in_localsearchs_sandbox() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("files-tracker-{}", std::process::id()));
    make_tree(&root);
    let probe = build_probe();
    let output = Command::new("localsearch")
        .arg("test-sandbox")
        .arg("--store-tmpdir")
        .arg("--index-recursive-directories")
        .arg(&root)
        .arg("--")
        .arg(&probe)
        .arg("--detect")
        .args([
            "--backend",
            "tracker",
            "--query",
            "M9",
            "--budget-ms",
            "2000",
        ])
        .args(["--retry-ms", "30000", "--home"])
        .arg(&root)
        .output()
        .unwrap_or_else(|e| panic!("running localsearch test-sandbox: {e}"));
    let _ = std::fs::remove_dir_all(&root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("sill: tracker sandbox said:\n{stdout}");
    // Running or Dormant, depending on whether the sandbox's miner holds its name yet when the
    // probe starts; a named `tracker` backend asks it either way (and may start it).
    assert!(
        stdout.contains("detect tracker=Running") || stdout.contains("detect tracker=Dormant"),
        "the sandbox's miner is not on its bus"
    );
    let rows: Vec<&str> = stdout.lines().filter(|l| l.starts_with("row\t")).collect();
    let mut sorted = rows.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        [
            "row\tm9 dir\t~\tinode/directory",
            "row\tm9-report.pdf\t~/Documents\tapplication/pdf",
        ],
        "{stdout}"
    );
}

fn make_tree(root: &Path) {
    let write = |relative: &str, text: &str| {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{e}"));
        }
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("{e}"));
    };
    write("Documents/m9-report.pdf", "%PDF-1.4\n%%EOF\n");
    write("notes/unrelated.md", "# nothing\n");
    std::fs::create_dir_all(root.join("m9 dir")).unwrap_or_else(|e| panic!("{e}"));
}

/// Build the probe and return its path, from cargo's own report of the executable.
fn build_probe() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "-p",
            "sill-testkit",
            "--example",
            "files_probe",
        ])
        .arg("--message-format=json")
        .output()
        .unwrap_or_else(|e| panic!("building files_probe: {e}"));
    assert!(output.status.success(), "building files_probe failed");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["target"]["name"] == "files_probe")
        .find_map(|message| message["executable"].as_str().map(PathBuf::from))
        .unwrap_or_else(|| panic!("cargo named no files_probe executable"))
}
