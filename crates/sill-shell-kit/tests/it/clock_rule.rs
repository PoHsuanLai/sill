//! The clock rule, held over the shell parts' sources: like quire's own (`ds/tests/clock_rule.rs`),
//! they read time only through `ds::base::time::clock` (`now`, `since`, `sleep`), so a harness's
//! virtual clock reaches every timer and every "now". Test modules (from a file's `#[cfg(test)]`
//! on, and `*tests.rs` files) are exempt; comments are skipped.

use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|error| panic!("{dir:?}: {error}"));
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| match path.is_dir() {
            true => sources(&path),
            false if path.extension().is_some_and(|ext| ext == "rs") => vec![path],
            false => Vec::new(),
        })
        .collect()
}

const FORBIDDEN: &[&str] = &["Instant::now", "futures_timer::", "Delay::new"];

/// The offending lines of one file's non-test code, as `path:line: text`.
fn offences(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    text.lines()
        .enumerate()
        .take_while(|(_, line)| !line.trim_start().starts_with("#[cfg(test)]"))
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| FORBIDDEN.iter().any(|needle| line.contains(needle)))
        .map(|(index, line)| format!("{}:{}: {}", path.display(), index + 1, line.trim()))
        .collect()
}

#[test]
fn the_shell_parts_read_time_only_through_ds_time() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found: Vec<String> = sources(&src)
        .into_iter()
        .filter(|path| {
            !path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("tests.rs"))
        })
        .flat_map(|path| offences(&path))
        .collect();
    assert!(
        found.is_empty(),
        "read the time through ds::base::time::clock::{{now, since, sleep}} instead:\n{}",
        found.join("\n")
    );
}
