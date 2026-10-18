//! The center's own quire controls (Clear All) show quire's focus ring
//! under the keyboard. The center runs the real `NotificationCenter`, whose root wraps quire's
//! `Ds` under the `HostSignals` that `mount` provides; with no shell-host `SurfaceHandle` (`ds_harness::Harness` has no
//! seat) the bridge provides nothing, and the harness's own root
//! `HostSignals` (driven by its `key`/`click`, `ds_blitz::launch`'s own mechanism) is what
//! `Ds` reads instead: the harness's way of driving modality, not shell-host's.

use super::{advance, opened, two_apps};
use ds::prelude::*;
use ds_harness::{FocusState, Harness, Input};

fn modality(harness: &Harness) -> Option<String> {
    harness.attr(".ds", "data-modality")
}

#[test]
fn a_key_turns_the_centers_ring_on_and_a_press_turns_it_off() {
    let mut running = opened(two_apps());
    assert_eq!(
        modality(&running.harness).as_deref(),
        Some("pointer"),
        "at first"
    );
    running.harness.send(Input::key(ShortcutKey::Tab));
    advance(&mut running, 20);
    assert_eq!(
        running.harness.focus_of(".ds-button"),
        FocusState::Focused,
        "Tab reaches Clear All"
    );
    assert_eq!(
        modality(&running.harness).as_deref(),
        Some("keyboard"),
        "after Tab"
    );
    let title = running
        .harness
        .centre(".sill-nc-head .ds-label")
        .expect("the title is laid out"); // test-only
    running.harness.send(Input::click(title));
    advance(&mut running, 20);
    assert_eq!(
        modality(&running.harness).as_deref(),
        Some("pointer"),
        "after a press"
    );
}
