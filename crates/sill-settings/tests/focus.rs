//! The focus link end to end: a set is written to `settings.toml`, the settings watch settles
//! it, and the mode follows (FINDINGS "M5 freeze", agent b).

mod common;

use std::time::Duration;

use common::TempDir;
use sill_settings::{Delivery, FocusMode, ShellFile, focus_mode, load, spawn_focus};
use tokio::sync::watch;

#[test]
fn a_set_is_stored_and_the_mode_follows_the_feed() {
    let dir = TempDir::new("focus");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let (feed_tx, feed) = watch::channel(ShellFile::default());
    let link = spawn_focus(rt.handle(), feed, Some(dir.path().to_owned()));
    let mut mode = link.mode();
    assert_eq!(*mode.borrow_and_update(), FocusMode::Off);

    assert_eq!(link.set(FocusMode::DoNotDisturb), Delivery::Queued);
    // The write lands in the file; the daemon's watch would then publish it.
    let stored = (0..50).find_map(|_| {
        std::thread::sleep(Duration::from_millis(20));
        let file = load(dir.path());
        (focus_mode(&file) == FocusMode::DoNotDisturb).then_some(file)
    });
    let stored = stored.unwrap_or_else(|| panic!("the set never reached settings.toml"));
    assert_eq!(
        *mode.borrow(),
        FocusMode::Off,
        "not before the feed settles"
    );

    feed_tx.send_replace(stored);
    rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(1), mode.changed())
            .await
            .unwrap_or_else(|_| panic!("the mode did not follow the feed"))
            .unwrap_or_else(|e| panic!("{e}"));
    });
    assert_eq!(*mode.borrow(), FocusMode::DoNotDisturb);
}
