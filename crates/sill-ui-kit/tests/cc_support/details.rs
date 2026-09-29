//! Helpers for the control center details tests (`cc_details_rows`, `cc_details_modules`):
//! the panel on the fake services, a press, a detail pane opened, a percentage read.

use std::time::Duration;

use ds_native::Harness;
use ds_native::harness::settle_until;

use super::{fake, live};

pub fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The panel on the fake services, the popover placed.
pub fn center(feed: impl FnOnce(&fake::Feeds)) -> live::Running {
    let (services, feeds) = fake::services();
    feed(&feeds);
    let mut running = live::start(services, feeds);
    running.harness.advance(ms(300));
    running
}

/// [`center`] on an output at `scale_percent`.
pub fn center_at(scale_percent: u16, feed: impl FnOnce(&fake::Feeds)) -> live::Running {
    let (services, feeds) = fake::services();
    feed(&feeds);
    let mut running = live::start_at(services, feeds, scale_percent);
    running.harness.advance(ms(300));
    running
}

/// Presses the element at `selector`.
pub fn press(harness: &mut Harness, selector: &str) {
    let at = harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is laid out: {}", harness.html()));
    harness.click(at);
}

/// Opens the detail pane behind the chevron named `label`.
pub fn open(harness: &mut Harness, label: &str) {
    press(harness, &format!("[*|aria-label=\"{label}\"]"));
    settle_until(harness, |h| {
        h.attr(".ds-panes", "data-shown").as_deref() == Some("detail")
            && h.count(".ds-module-grid") == 0
    });
}

/// The percentage `selector` shows, without its `%`.
pub fn figure(harness: &Harness, selector: &str) -> Option<u32> {
    harness
        .text_of(selector)
        .and_then(|text| text.trim_end_matches('%').parse().ok())
}
