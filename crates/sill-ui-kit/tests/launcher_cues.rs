//! quire's launcher cues as the real panel plays them, on the virtual clock: the pane enters
//! and rests, another kind cross-fades the media in place, a slow text head shows "Loading…" at
//! once and keeps turning for as long as it reads, a later result set replaces in place, Show
//! More enters its rows and Show Less heals. Every step but the pending one ends at 0 frames.

mod launcher_harness;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use ds::ShortcutKey;
use ds_native::Harness;
use ds_native::harness::assert_settles_to_zero_frames;
use launcher_harness::launcher_support::m9;
use launcher_harness::{Open, Press, Seen, await_off_thread, loaded, open, settle, watch};
use sill_launcher::PreviewKind;

/// Show the preview pane with ⌘Y through the harness's keyboard.
fn quick_look(open: &mut Open) {
    open.press(Press::QuickLook);
    open.harness.advance(Duration::ZERO);
}

/// Press `key` on the harness's keyboard.
fn press(open: &mut Open, key: Press) {
    open.press(key);
    open.harness.advance(Duration::ZERO);
}

fn entrance(harness: &Harness) -> Option<String> {
    harness.attr(".ds-preview", "data-pulse")
}

fn fading(harness: &Harness) -> bool {
    harness.has_class(".ds-preview-media", "a-morph-fade-in")
}

fn revealing(harness: &Harness) -> bool {
    harness.attr(".ds-menu", "data-reveal").is_some()
}

#[test]
fn the_pane_enters_on_a_real_key_and_rests() {
    let mut open = open("m9");
    assert_settles_to_zero_frames(&mut open.harness);
    quick_look(&mut open);
    settle(&mut open.harness, |h| h.count(".ds-preview") == 1);
    assert_eq!(
        await_off_thread(&mut open.harness, loaded),
        Seen::Once,
        "the page lands"
    );
    assert_settles_to_zero_frames(&mut open.harness);
    assert_eq!(open.harness.attr(".ds-preview", "data-touch"), None);
}

#[test]
fn another_kind_cross_fades_and_never_replays_the_entrance() {
    let mut open = open("m9");
    quick_look(&mut open);
    assert_eq!(
        await_off_thread(&mut open.harness, loaded),
        Seen::Once,
        "the page lands"
    );
    assert_eq!(open.harness.count(".ds-pdf-thumb"), 1);
    assert_settles_to_zero_frames(&mut open.harness);
    assert_eq!(
        open.link.rows().and_then(|r| r.preview),
        Some(PreviewKind::Pdf)
    );
    let before = entrance(&open.harness);

    // Down onto the PNG: the selection moved onto another kind (a Preview moment).
    press(&mut open, Press::Down);
    assert_eq!(
        open.link.rows().and_then(|r| r.preview),
        Some(PreviewKind::Image)
    );
    assert_eq!(
        watch(&mut open.harness, fading),
        Seen::Once,
        "the media fades in"
    );
    assert_eq!(
        entrance(&open.harness),
        before,
        "the pane's entrance did not play again"
    );
    assert_eq!(
        open.harness.count(".ds-preview"),
        1,
        "one pane, still mounted"
    );
    assert_settles_to_zero_frames(&mut open.harness);
    assert!(!fading(&open.harness), "the fade settled");
    assert_eq!(entrance(&open.harness), before);
}

/// The FIFO the slow text file's head is read from: its read waits until the test writes it.
struct SlowHead<'a>(&'a Path);

impl SlowHead<'_> {
    fn make(path: &Path) -> SlowHead<'_> {
        let _ = std::fs::remove_file(path);
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo runs"); // test-only
        assert!(made.success(), "mkfifo {}", path.display());
        SlowHead(path)
    }

    /// Write `text` as the head and close the FIFO, so the read lands.
    fn land(&self, text: &'static str) {
        let path = self.0.to_path_buf();
        std::thread::spawn(move || {
            let mut fifo = std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .expect("the FIFO opens for writing"); // test-only
            fifo.write_all(text.as_bytes())
                .expect("the head is written"); // test-only
        });
    }
}

impl Drop for SlowHead<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0);
    }
}

#[test]
fn a_slow_text_head_shows_loading_at_once_then_the_text() {
    let path = m9::slow_head_path();
    let head = SlowHead::make(&path);
    let mut open = open("slow-head");
    press(&mut open, Press::QuickLook);
    settle(&mut open.harness, |h| h.count(".ds-preview") == 1);
    assert_eq!(
        open.link.rows().and_then(|r| r.preview),
        Some(PreviewKind::TextFile)
    );
    // No grace: the pending look is there from the first frames, and the pane is busy.
    settle(&mut open.harness, |h| h.count(".ds-preview-pending") == 1);
    assert_eq!(
        open.harness.attr(".ds-preview", "aria-busy").as_deref(),
        Some("true")
    );
    assert!(
        open.harness
            .text_of(".ds-preview-pending")
            .is_some_and(|text| text.contains("Loading\u{2026}")),
        "the words say so"
    );
    // No cap: it keeps turning for as long as the read runs.
    open.harness.advance(Duration::from_secs(10));
    assert_eq!(open.harness.count(".ds-preview-pending"), 1);

    head.land("The head landed.\n");
    let landed = await_off_thread(&mut open.harness, |h| {
        h.text_of(".ds-preview-media")
            .is_some_and(|text| text.contains("The head landed."))
    });
    assert_eq!(landed, Seen::Once, "the text replaces the pending look");
    assert_eq!(open.harness.count(".ds-preview-pending"), 0);
    assert_eq!(open.harness.attr(".ds-preview", "aria-busy"), None);
    assert_settles_to_zero_frames(&mut open.harness);
}

#[test]
fn a_result_set_replaces_in_place_with_no_reveal() {
    let mut open = open("m9");
    assert!(
        !revealing(&open.harness),
        "the opening's first set is simply there"
    );
    assert_settles_to_zero_frames(&mut open.harness);

    // A new answer (typed): replaced in place.
    open.harness.key(ShortcutKey::Backspace);
    let rose = watch(&mut open.harness, revealing);
    assert_eq!(rose, Seen::Never, "a later set plays nothing");
    assert_eq!(
        open.link.rows().map(|r| r.query).as_deref(),
        Some("m"),
        "the query changed"
    );
    assert_settles_to_zero_frames(&mut open.harness);
}

fn row_motion(harness: &Harness, motion: &str) -> bool {
    harness.count(&format!("[*|data-row-motion={motion}]")) > 0
}

#[test]
fn show_more_enters_its_rows_and_show_less_heals() {
    let mut open = open("m9");
    assert_settles_to_zero_frames(&mut open.harness);
    // Top Hit, five documents, then the Show More line (6): the person's Down and Enter.
    for _ in 0..6 {
        open.harness.key(ShortcutKey::Down);
    }
    open.harness.advance(Duration::from_millis(20));
    assert_eq!(open.link.rows().map(|r| r.selected), Some(6));
    open.harness.key(ShortcutKey::Enter);
    let rose = watch(&mut open.harness, |h| row_motion(h, "in"));
    assert_eq!(rose, Seen::Once, "the added rows enter");
    assert_eq!(
        open.link.rows().map(|r| r.sections[1].hidden),
        Some(0),
        "all seven show"
    );
    assert_settles_to_zero_frames(&mut open.harness);
    assert!(!row_motion(&open.harness, "in"), "the entrance settled");

    // The selection followed its line, now Show Less (8): Enter again, the rows after heal up.
    assert_eq!(open.link.rows().map(|r| r.selected), Some(8));
    open.harness.key(ShortcutKey::Enter);
    let healed = watch(&mut open.harness, |h| row_motion(h, "heal-from"));
    assert_eq!(healed, Seen::Once, "what follows heals up");
    let shown = open.link.rows();
    assert_eq!(
        shown.as_ref().map(|r| r.sections[1].hidden),
        Some(2),
        "capped again"
    );
    assert_eq!(shown.map(|r| r.selected), Some(6), "on Show More again");
    assert_settles_to_zero_frames(&mut open.harness);
}

/// The same Show More through the link (`sill debug launcher-key`): sill runs the group's
/// action itself and marks it on quire's palette handle, so the rows still enter.
#[test]
fn a_show_more_through_the_link_still_enters() {
    let mut open = open("m9");
    assert_settles_to_zero_frames(&mut open.harness);
    for _ in 0..6 {
        press(&mut open, Press::Down);
    }
    assert_eq!(open.link.rows().map(|r| r.selected), Some(6));
    open.press(Press::Enter);
    let rose = watch(&mut open.harness, |h| row_motion(h, "in"));
    assert_eq!(rose, Seen::Once, "the added rows enter");
    assert_settles_to_zero_frames(&mut open.harness);
}
