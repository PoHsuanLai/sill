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
