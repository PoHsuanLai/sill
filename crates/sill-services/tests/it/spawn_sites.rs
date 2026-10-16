//! Where this crate may start a process. A test must never open a file, trash one or start an app
//! in the person's own session (a stray `kde-open` held a gate's pipe for forty minutes), so every
//! child that does something on the desktop goes through `Env::children`, which an isolated `Env`
//! records instead of running. A new `Command::new` anywhere else fails here until it is either
//! routed through that seam or added to the list with the reason it is safe.

use std::fs;
use std::path::{Path, PathBuf};

/// The files that start a process of their own, and why each is not the opener's seam.
const ALLOWED: [(&str, &str); 6] = [
    ("env/children.rs", "the seam's real implementation"),
    (
        "launch/backend/run.rs",
        "an app launch, tracked to its exit; only an `#[ignore]`d test runs it",
    ),
    (
        "now_playing/backend/art/curl.rs",
        "curl into the cache directory",
    ),
    (
        "screenshots/backend/child.rs",
        "the capture tool, a stand-in in every test",
    ),
    (
        "auth/backend/helper.rs",
        "the PAM helper, replaced by `FakeAuthenticator` in `Env::isolated`",
    ),
    (
        "polkit_agent/backend/helper_run.rs",
        "polkit's helper, `HelperRoute::Missing` in `Env::isolated`",
    ),
];

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn only_the_listed_files_start_a_process() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    let strays: Vec<String> = files
        .iter()
        .filter(|path| fs::read_to_string(path).is_ok_and(|text| text.contains("Command::new(")))
        .filter_map(|path| path.strip_prefix(&src).ok())
        .map(|relative| relative.display().to_string())
        .filter(|relative| !ALLOWED.iter().any(|(allowed, _)| allowed == relative))
        .collect();
    assert!(
        strays.is_empty(),
        "starts a process outside `Env::children` and the list in this test: {strays:?}"
    );
}
