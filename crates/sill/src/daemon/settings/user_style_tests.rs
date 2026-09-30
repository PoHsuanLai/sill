//! The stylesheet's path to the feed: a watched text is published, every other event leaves the
//! feed alone, a file write reaches it through the real watch, and a reload re-reads the file.

use std::time::Duration;

use ds_blitz::TokioSpawner;
use ds_settings::UserStyle;
use shell_host::HostEvent;
use tokio::sync::watch;

use super::{load_user_style, publish_watched_style, reload_user_style};
use crate::daemon::msg::ShellMsg;
use crate::daemon::settings::store::store;

fn style(text: &str) -> UserStyle {
    UserStyle(text.to_owned())
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sill-daemon-user-style-test-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

#[test]
fn a_watched_style_event_publishes_its_text() {
    let (tx, rx) = watch::channel(UserStyle::default());
    let event = HostEvent::App(ShellMsg::UserStyle(Box::new(style(".ds { }"))));
    publish_watched_style(&tx, &event);
    assert_eq!(rx.borrow().clone(), style(".ds { }"));
}

#[test]
fn every_other_event_leaves_the_style_alone() {
    let (tx, rx) = watch::channel(style(".kept { }"));
    for event in [
        HostEvent::Started,
        HostEvent::App(ShellMsg::Dim(sill_model::idle::DimShown::Clear)),
    ] {
        publish_watched_style(&tx, &event);
    }
    assert_eq!(rx.borrow().clone(), style(".kept { }"));
}

#[test]
fn a_reload_re_reads_the_file_and_a_deleted_file_is_an_empty_style() {
    let dir = temp_dir("reload");
    let store = store(&dir);
    let (tx, rx) = watch::channel(load_user_style(&store));
    assert_eq!(rx.borrow().clone(), UserStyle::default(), "no file yet");
    store
        .save(&style(".a { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    reload_user_style(&store, &tx);
    assert_eq!(rx.borrow().clone(), style(".a { }"));
    let path = store
        .path::<UserStyle>()
        .unwrap_or_else(|| panic!("no path"));
    std::fs::remove_file(path).unwrap_or_else(|e| panic!("{e}"));
    reload_user_style(&store, &tx);
    assert_eq!(rx.borrow().clone(), UserStyle::default());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `style.css` write under a temp config dir, with no `sill reload`, reaches the feed through
/// the same watch and the same publish the host event handler calls.
#[tokio::test]
async fn a_file_write_reaches_the_feed_without_a_reload() {
    let dir = temp_dir("live");
    let store = store(&dir);
    let mut watch = store.watch::<UserStyle>(&TokioSpawner::current());
    let (tx, rx) = watch::channel(UserStyle::default());

    store
        .save(&style(".b { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    let loaded = tokio::time::timeout(Duration::from_secs(2), watch.changed())
        .await
        .unwrap_or_else(|_| panic!("no settled change within 2s"))
        .unwrap_or_else(|| panic!("the watch stopped"));
    publish_watched_style(
        &tx,
        &HostEvent::App(ShellMsg::UserStyle(Box::new(loaded.value))),
    );

    assert_eq!(rx.borrow().clone(), style(".b { }"));
    let _ = std::fs::remove_dir_all(&dir);
}
