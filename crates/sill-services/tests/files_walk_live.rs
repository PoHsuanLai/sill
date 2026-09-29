//! The walking index and detection for real, on this machine, never over the person's files:
//! `fd` walks a scratch folder under the target directory, and `detect` only asks whether each
//! backend exists (a bus name, a program on `PATH`).
//!
//! Run explicitly: `cargo test -p sill-services --test files_walk_live -- --ignored
//! --nocapture`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sill_launcher::{Cancel, Query};
use sill_services::providers::files::choice::IndexState;
use sill_services::providers::files::choice::detect;
use sill_services::providers::files::walk::{WalkIndex, Walker};
use sill_services::providers::files::{FileAnswer, FileAsk, FileIndex, HitLimit};

fn scratch(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("files-walk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (relative, text) in [
        ("Documents/m9-report.pdf", "%PDF-1.4\n"),
        ("notes/m9-notes.md", "# m9\n"),
        (".hidden/m9-secret.txt", "hidden\n"),
        ("other/unrelated.txt", "x\n"),
    ] {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{e}"));
        }
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("{e}"));
    }
    root
}

fn ask(text: &str, under: &Path, budget: Duration) -> FileAsk {
    FileAsk {
        text: text.to_owned(),
        under: under.to_path_buf(),
        limit: HitLimit(40),
        budget,
    }
}

#[test]
#[ignore = "needs fd"]
fn fd_finds_names_under_the_folder_and_skips_hidden_ones() {
    let root = scratch("fd");
    let index = WalkIndex { walker: Walker::Fd };
    let query = Query::begin("M9", &Cancel::new());
    let answer = index.search(&ask("M9", &root, Duration::from_secs(5)), &query);
    let _ = std::fs::remove_dir_all(&root);
    let FileAnswer::Hits(hits) = answer else {
        panic!("fd did not finish: {answer:?}");
    };
    let mut names: Vec<(String, String)> = hits
        .iter()
        .map(|h| (h.name.clone(), h.mime.0.clone()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            ("m9-notes.md".to_owned(), "text/markdown".to_owned()),
            ("m9-report.pdf".to_owned(), "application/pdf".to_owned()),
        ]
    );
    assert!(hits.iter().all(|h| h.path.starts_with(&root)));
    assert!(hits.iter().all(|h| h.modified.0 > 0));
}

#[test]
#[ignore = "needs fd"]
fn a_budget_of_nothing_is_a_partial_answer() {
    let root = scratch("budget");
    let index = WalkIndex { walker: Walker::Fd };
    let query = Query::begin("m9", &Cancel::new());
    let answer = index.search(&ask("m9", &root, Duration::ZERO), &query);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(answer, FileAnswer::Partial(Vec::new()));
}

/// Step 5 of the lane's acceptance: detection only, reported (FINDINGS "M9 lane a").
#[test]
#[ignore = "needs tracker"]
fn this_machine_has_tracker() {
    let found = detect();
    eprintln!(
        "sill: files detect: tracker={:?} fd={:?} plocate={:?}",
        found.tracker, found.fd, found.plocate
    );
    assert_ne!(found.tracker, IndexState::Absent);
}
