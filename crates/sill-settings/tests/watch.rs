//! design/22-SETTINGS.md §6 test 3: an atomic temp-and-rename write fires exactly one settled
//! change within the 30 ms debounce plus a margin.

mod common;

use std::time::Duration;

use common::TempDir;
use ds_settings::{DEBOUNCE, Px};
use sill_settings::{ShellFile, save, watch};

/// Generous slack over the debounce for a loaded test runner.
const MARGIN: Duration = Duration::from_millis(2_000);

#[tokio::test]
async fn a_renamed_write_fires_exactly_one_change() {
    let dir = TempDir::new("watch");
    save(dir.path(), &ShellFile::default()).unwrap_or_else(|e| panic!("{e}"));
    let mut shell_watch = watch(dir.path()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(shell_watch.current(), ShellFile::default());

    let mut changed = ShellFile::default();
    changed.bar.height_px = Px(28);
    save(dir.path(), &changed).unwrap_or_else(|e| panic!("{e}"));

    let got = tokio::time::timeout(MARGIN, shell_watch.changed())
        .await
        .unwrap_or_else(|_| panic!("no change within the debounce plus margin"))
        .unwrap_or_else(|| panic!("the watch stopped"));
    assert_eq!(got, changed);
    assert_eq!(shell_watch.receiver().borrow().clone(), changed);

    let extra = tokio::time::timeout(DEBOUNCE * 4, shell_watch.changed()).await;
    assert!(extra.is_err(), "expected no further change, got {extra:?}");
}

/// A rename onto `settings.toml` that is not TOML at all: the watch keeps the last good file rather
/// than blanking it to defaults, and a later good write is still picked up.
#[tokio::test]
async fn an_unparseable_write_keeps_the_last_good_file() {
    let dir = TempDir::new("watch-bad");
    let mut good = ShellFile::default();
    good.bar.height_px = Px(30);
    save(dir.path(), &good).unwrap_or_else(|e| panic!("{e}"));
    let mut shell_watch = watch(dir.path()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(shell_watch.current(), good);

    // The same temp-and-rename an editor or `save` uses, but with a syntax error.
    let path = dir.path().join("settings.toml");
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, "not toml [[[ = =").unwrap_or_else(|e| panic!("{e}"));
    std::fs::rename(&tmp, &path).unwrap_or_else(|e| panic!("{e}"));

    let extra = tokio::time::timeout(MARGIN, shell_watch.changed()).await;
    assert!(
        extra.is_err(),
        "an unparseable file must not settle a change, got {extra:?}"
    );
    assert_eq!(
        shell_watch.current(),
        good,
        "the last good file is kept while the file on disk is broken"
    );

    let mut fixed = good.clone();
    fixed.bar.height_px = Px(40);
    save(dir.path(), &fixed).unwrap_or_else(|e| panic!("{e}"));
    let got = tokio::time::timeout(MARGIN, shell_watch.changed())
        .await
        .unwrap_or_else(|_| panic!("no change within the debounce plus margin"))
        .unwrap_or_else(|| panic!("the watch stopped"));
    assert_eq!(got, fixed, "a later good write is picked up");
}
