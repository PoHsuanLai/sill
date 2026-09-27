//! FINDINGS G390/F74: the center's own quire controls (Clear All) now show quire's focus ring
//! under the keyboard. The center runs the real `NotificationCenter`, whose root wraps quire's
//! `Ds` in `HostModalityBridge`; with no shell-host `SurfaceHandle` (`ds_native::Harness` has no
//! seat) the bridge no-ops, same as `use_ds_scale`'s fallback, and the harness's own root
//! `HostModality` (driven by its `key`/`click`, `ds_native::launch`'s own mechanism) is what
//! `Ds` reads instead: the harness's way of driving modality, not shell-host's.

mod center_support;

use center_support::{opened, two_apps};
use ds::Key;
use ds_native::Harness;

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
    running.harness.key(Key::Tab);
    running.advance(20);
    assert!(
        running.harness.is_focused(".ds-button"),
        "Tab reaches Clear All"
    );
    assert_eq!(
        modality(&running.harness).as_deref(),
        Some("keyboard"),
        "after Tab"
    );
    let title = running
        .harness
        .centre(".sill-nc-title")
        .expect("the title is laid out"); // test-only
    running.harness.click(title);
    running.advance(20);
    assert_eq!(
        modality(&running.harness).as_deref(),
        Some("pointer"),
        "after a press"
    );
}
