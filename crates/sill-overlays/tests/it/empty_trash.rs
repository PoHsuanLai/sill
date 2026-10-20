//! What the Empty Trash question draws and sends, on the harness over an `EmptyTrashLink` the
//! test writes as the daemon does: nothing until it is opened; then the Mac's alert with Cancel
//! (the default, and Escape) and Empty Trash; only Empty Trash writes a yes, once, and every
//! answer takes the sheet down; nothing answers before the arming time.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use sill_overlays::EmptyTrashSheet;
use sill_overlays::arming::QUESTION_ARM_AFTER;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::EmptyTrashLink;

type Running = SurfaceHarness<EmptyTrashSheet>;

const CANCEL: &str = "#empty-trash-cancel";
const EMPTY: &str = "#empty-trash-confirm";

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The question asked, once it takes answers.
fn up() -> Running {
    let mut running = SurfaceHarness::<EmptyTrashSheet>::setup(EmptyTrashLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running.link.open(fake::output().id);
    settle(&mut running.harness, |h| h.count(".ds-alert") == 1);
    running
        .harness
        .advance(QUESTION_ARM_AFTER + Duration::from_millis(100));
    running
}

fn click(running: &mut Running, selector: &str) {
    let at = running
        .harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not laid out: {}", running.harness.html()));
    running.harness.send(Input::click(at));
    running.harness.advance(Duration::from_millis(40));
}

#[test]
fn until_it_is_opened_nothing_is_drawn() {
    let mut running = SurfaceHarness::<EmptyTrashSheet>::setup(EmptyTrashLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn it_asks_in_the_macs_words_with_cancel_and_empty_trash() {
    let running = up();
    let html = running.harness.html();
    for word in [
        "Are you sure you want to permanently erase the items in the Trash?",
        "t undo this action.",
        "Cancel",
        "Empty Trash",
    ] {
        assert!(html.contains(word), "{word}: {html}");
    }
    assert_eq!(running.harness.count(CANCEL), 1, "{html}");
    assert_eq!(running.harness.count(EMPTY), 1, "{html}");
}

#[test]
fn only_empty_trash_says_yes_and_either_takes_the_question_down() {
    let mut cancelled = up();
    click(&mut cancelled, CANCEL);
    assert_eq!(cancelled.link.last_answered(), None);
    settle(&mut cancelled.harness, |h| h.count(".ds-alert") == 0);

    let mut erased = up();
    click(&mut erased, EMPTY);
    assert_eq!(erased.link.last_answered().map(|sent| sent.seq), Some(1));
    settle(&mut erased.harness, |h| h.count(".ds-alert") == 0);
}

#[test]
fn escape_and_return_both_cancel() {
    for key in [ShortcutKey::Escape, ShortcutKey::Enter] {
        let mut running = up();
        running.harness.send(Input::key(key));
        running.harness.advance(Duration::from_millis(40));
        assert_eq!(running.link.last_answered(), None);
    }
}
