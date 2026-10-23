//! The styles' path to the feeds: a watched text is published to its own sheet, every other event
//! leaves the feeds alone, a file write reaches them through the real watch, a reload re-reads both
//! files, and the crash guard turns the sheets off after two starts that never got a frame up and
//! lets them back on at Turn On Again or at the next change.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ds::style::kit::{Styles, UserStyle};
use ds_blitz::StylesGuard;
use ds_blitz::TokioSpawner;
use ds_blitz::styles_guard::Load;
use ds_settings::{AppearanceFile, Store};
use shell_host::HostEvent;

use super::StyleKeeper;
use crate::daemon::msg::{Sheet, ShellMsg};
use crate::daemon::settings::store::{desktop_store, store};

fn style(text: &str) -> UserStyle {
    UserStyle(text.to_owned())
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sill-daemon-styles-test-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

fn keeper(dir: &Path) -> StyleKeeper {
    StyleKeeper::start(
        desktop_store(dir),
        store(dir),
        desktop_store(dir),
        &dir.join("state"),
    )
}

fn styles_of(keeper: &StyleKeeper) -> Styles {
    keeper.feeds().0.borrow().clone()
}

fn load_of(keeper: &StyleKeeper) -> Option<Load> {
    keeper.feeds().1.borrow().as_ref().map(StylesGuard::load)
}

fn watched(sheet: Sheet, text: &str) -> HostEvent<ShellMsg> {
    HostEvent::App(ShellMsg::Style(sheet, Box::new(style(text))))
}

#[test]
fn a_watched_style_event_publishes_its_own_sheet() {
    let dir = temp_dir("publish");
    let keeper = keeper(&dir);
    keeper.publish(&watched(Sheet::Desktop, ".d { }"));
    keeper.publish(&watched(Sheet::App, ".a { }"));
    let now = styles_of(&keeper);
    assert_eq!((now.desktop, now.app), (style(".d { }"), style(".a { }")));
    assert!(now.look.is_blank(), "the look stays empty for now");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_other_event_leaves_the_styles_alone() {
    let dir = temp_dir("others");
    let keeper = keeper(&dir);
    keeper.publish(&watched(Sheet::App, ".kept { }"));
    for event in [
        HostEvent::Started,
        HostEvent::App(ShellMsg::Dim(sill_model::idle::DimShown::Clear)),
    ] {
        keeper.publish(&event);
    }
    assert_eq!(styles_of(&keeper).app, style(".kept { }"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_reload_re_reads_both_files_and_a_deleted_file_is_an_empty_style() {
    let dir = temp_dir("reload");
    let (wide, own) = (desktop_store(&dir), store(&dir));
    let keeper = keeper(&dir);
    assert_eq!(styles_of(&keeper).app, UserStyle::default(), "no file yet");
    wide.save(&style(".d { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    own.save(&style(".a { }")).unwrap_or_else(|e| panic!("{e}"));
    keeper.reload();
    let now = styles_of(&keeper);
    assert_eq!((now.desktop, now.app), (style(".d { }"), style(".a { }")));
    assert_eq!(
        now.homes.desktop,
        wide.dir(),
        "each may read its own folder"
    );
    assert_eq!(now.homes.app, own.dir());
    for store in [&wide, &own] {
        let path = store
            .path::<UserStyle>()
            .unwrap_or_else(|| panic!("no path"));
        std::fs::remove_file(path).unwrap_or_else(|e| panic!("{e}"));
    }
    keeper.reload();
    let now = styles_of(&keeper);
    assert_eq!(
        (now.desktop, now.app),
        (UserStyle::default(), UserStyle::default())
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A write to the desktop-wide `style.css` under a temp config dir, with no `sill reload`, reaches
/// the feed through the same watch and the same publish the host event handler calls.
#[tokio::test]
async fn a_desktop_wide_file_write_reaches_the_feed_without_a_reload() {
    let dir = temp_dir("live");
    let wide = desktop_store(&dir);
    let keeper = keeper(&dir);
    let mut watch = wide.watch::<UserStyle>(&TokioSpawner::current());

    wide.save(&style(".b { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    let loaded = tokio::time::timeout(Duration::from_secs(2), watch.changed())
        .await
        .unwrap_or_else(|_| panic!("no settled change within 2s"))
        .unwrap_or_else(|| panic!("the watch stopped"));
    keeper.publish(&watched(Sheet::Desktop, &loaded.value.0));

    assert_eq!(styles_of(&keeper).desktop, style(".b { }"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two starts that never marked themselves good turn the third off; Turn On Again writes
/// `appearance.custom_styles = "use"` and begins afresh.
#[test]
fn two_unfinished_starts_turn_the_styles_off_and_turn_on_again_brings_them_back() {
    let dir = temp_dir("guard");
    desktop_store(&dir)
        .save(&style(".d { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(load_of(&keeper(&dir)), Some(Load::WithStyles));
    assert_eq!(load_of(&keeper(&dir)), Some(Load::WithStyles));
    let third = keeper(&dir);
    assert_eq!(load_of(&third), Some(Load::Without));

    third.publish(&HostEvent::App(ShellMsg::TurnOnStyles));
    assert_eq!(load_of(&third), Some(Load::WithStyles));
    let appearance = text_of::<AppearanceFile>(&desktop_store(&dir));
    assert!(
        appearance.contains("custom_styles = \"use\""),
        "{appearance}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_change_to_any_sheet_gives_a_turned_off_start_a_fresh_try() {
    let dir = temp_dir("fix");
    desktop_store(&dir)
        .save(&style(".d { }"))
        .unwrap_or_else(|e| panic!("{e}"));
    keeper(&dir);
    keeper(&dir);
    let third = keeper(&dir);
    assert_eq!(load_of(&third), Some(Load::Without));
    third.publish(&watched(Sheet::Desktop, ".fixed { }"));
    assert_eq!(load_of(&third), Some(Load::WithStyles));
    let _ = std::fs::remove_dir_all(&dir);
}

fn text_of<D: ds_settings::SettingsDoc>(store: &Store) -> String {
    let path = store.path::<D>().unwrap_or_else(|| panic!("no path"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
