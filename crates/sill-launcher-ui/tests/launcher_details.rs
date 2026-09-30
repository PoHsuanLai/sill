//! The launcher's moments end (design/26-DETAILS.md R3, the idle-frame rule): opened, its
//! preview pane shown and hidden again, the panel settles to 0 frames, with no CSS animation
//! running and no Rust timer waking the document (`ds_harness::Harness::wakes`).

mod launcher_harness;

use std::time::Duration;

use ds_harness::harness::assert_settles_to_zero_frames;
use launcher_harness::{Open, Press, Seen, await_off_thread, loaded, open, settle};

/// Press `key` on the harness's keyboard.
fn press(open: &mut Open, key: Press) {
    open.press(key);
    open.harness.advance(Duration::from_millis(40));
}

#[test]
fn opened_the_panel_settles_to_zero_frames() {
    let mut open = open("m9");
    assert_settles_to_zero_frames(&mut open.harness);
}

#[test]
fn open_preview_hide_then_zero_frames_at_rest() {
    let mut open = open("m9");
    assert_settles_to_zero_frames(&mut open.harness);

    press(&mut open, Press::QuickLook);
    settle(&mut open.harness, |h| h.count(".ds-preview") > 0);
    assert_eq!(open.harness.count(".ds-preview"), 1, "the pane is up");
    // The PDF's page is rasterised in wall time; until it lands the pane's pending look runs.
    assert_eq!(
        await_off_thread(&mut open.harness, loaded),
        Seen::Once,
        "the page lands"
    );
    assert_settles_to_zero_frames(&mut open.harness);

    // Down onto the next row while it shows: the content changes in place, no entrance replays.
    press(&mut open, Press::Down);
    assert_settles_to_zero_frames(&mut open.harness);

    press(&mut open, Press::QuickLook);
    settle(&mut open.harness, |h| h.count(".ds-preview") == 0);
    assert_eq!(open.harness.count(".ds-preview"), 0, "the pane has left");
    assert_settles_to_zero_frames(&mut open.harness);
}
