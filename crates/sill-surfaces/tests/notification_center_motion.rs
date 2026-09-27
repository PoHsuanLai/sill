//! The notification center's small motions (design/26 5.4, design/20 1.6; FINDINGS "Manual run
//! 2026-09-28: notifications"), on the virtual clock with a pinned appearance: the rows rise
//! once as the center opens (quire's `Reveal`), the cards an opened group adds rise under its
//! head, and every one of them ends at rest, asking for no frame. Clear and Clear All reach the
//! service and the center then rests; their exit and the rows' heal wait on quire (Q510).

mod center_support;

use center_support::{fake, settle, start, two_apps};
use ds_native::harness::assert_settles_to_zero_frames;
use sill_services::{AppKey, Clear, History, NotificationsCommand};
use sill_settings::ShellFile;

/// A group's toggle ("N more", "Show less") and Clear, in its header.
const TOGGLE: &str = ".ds-group-header-actions .ds-button:first-child";
const CLEAR: &str = ".ds-group-header-actions .ds-button:last-child";

/// The list's own rise: quire's `Reveal` round the groups.
const LIST_REVEAL: &str = ".sill-nc-list > .ds-reveal";

/// The rise of the cards an opened group adds.
const GROUP_REVEAL: &str = ".sill-nc-group .ds-reveal";

/// The center opened over `history`, as soon as its panel is drawn.
fn opening(history: History) -> center_support::Running {
    let mut running = start(ShellFile::default(), history);
    running.advance(40);
    running.link.open(fake::output().id);
    settle(&mut running.harness, |h| h.count(".ds-panel") == 1);
    running
}

#[test]
fn opening_rises_the_rows_once_then_rests() {
    let mut running = opening(two_apps());
    let h = &running.harness;
    assert_eq!(
        h.attr(LIST_REVEAL, "data-reveal").as_deref(),
        Some("play"),
        "the rows rise as the center opens"
    );
    assert_eq!(
        h.count(&format!("{LIST_REVEAL} > .sill-nc-group")),
        2,
        "each group is one of the rise's children"
    );
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(
        running.harness.attr(LIST_REVEAL, "data-reveal").as_deref(),
        Some("still"),
        "and never replays"
    );
    // A re-render (a new history) replays nothing.
    running
        .feeds
        .notification_history
        .send_replace(center_support::two_apps());
    running.advance(40);
    assert_eq!(
        running.harness.attr(LIST_REVEAL, "data-reveal").as_deref(),
        Some("still")
    );
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn opening_a_group_rises_the_cards_it_adds_then_rests() {
    let mut running = opening(two_apps());
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(
        running.harness.count(GROUP_REVEAL),
        0,
        "folded: nothing added"
    );
    let toggle = format!(".sill-nc-group:nth-child(1) {TOGGLE}");
    let more = running.harness.centre(&toggle).expect("N more");
    running.harness.click(more);
    running.until(|h| h.count(".sill-nc-row") == 4);
    let h = &running.harness;
    assert_eq!(
        h.attr(GROUP_REVEAL, "data-reveal").as_deref(),
        Some("play"),
        "the added cards rise under the head"
    );
    assert_eq!(
        h.count(&format!("{GROUP_REVEAL} > .sill-nc-row")),
        2,
        "Mail's two older cards, not its newest"
    );
    assert_settles_to_zero_frames(&mut running.harness);
    assert_eq!(
        running.harness.attr(GROUP_REVEAL, "data-reveal").as_deref(),
        Some("still")
    );
    let less = running.harness.centre(&toggle).expect("Show less");
    running.harness.click(less);
    running.until(|h| h.count(".sill-nc-row") == 2);
    assert_eq!(running.harness.count(GROUP_REVEAL), 0);
    assert_settles_to_zero_frames(&mut running.harness);
}

#[test]
fn the_added_cards_keep_the_lists_gap() {
    let mut running = opening(two_apps());
    assert_settles_to_zero_frames(&mut running.harness);
    let toggle = format!(".sill-nc-group:nth-child(1) {TOGGLE}");
    let more = running.harness.centre(&toggle).expect("N more");
    running.harness.click(more);
    running.until(|h| h.count(".sill-nc-row") == 4);
    assert_settles_to_zero_frames(&mut running.harness);
    let h = &running.harness;
    let gap = |upper: &str, lower: &str| {
        let (a, b) = (h.rect(upper).expect(upper), h.rect(lower).expect(lower));
        b.origin.y.0 - (a.origin.y.0 + a.size.height.0)
    };
    let newest = ".sill-nc-group:nth-child(1) > .sill-nc-row";
    let second = ".sill-nc-group:nth-child(1) .ds-reveal > .sill-nc-row:nth-child(1)";
    let third = ".sill-nc-group:nth-child(1) .ds-reveal > .sill-nc-row:nth-child(2)";
    let (one, two) = (gap(newest, second), gap(second, third));
    assert!(one > 0.5 && (one - two).abs() < 0.5, "{one} {two}");
    let groups = gap(".sill-nc-group:nth-child(1)", ".sill-nc-group:nth-child(2)");
    assert!((groups - one).abs() < 0.5, "{groups} {one}");
}

#[test]
fn clear_and_clear_all_reach_the_service_and_the_center_rests() {
    let mut running = opening(two_apps());
    assert_settles_to_zero_frames(&mut running.harness);
    let clear = running
        .harness
        .centre(&format!(".sill-nc-group:nth-child(2) {CLEAR}"))
        .expect("Chat's Clear");
    running.harness.click(clear);
    assert_eq!(
        running.commands_until(1),
        vec![NotificationsCommand::ClearHistory(Clear::App(AppKey(
            "chat".to_owned()
        )))]
    );
    let mut left = two_apps();
    left.groups.truncate(1);
    running.feeds.notification_history.send_replace(left);
    running.until(|h| h.count(".sill-nc-group") == 1);
    assert_settles_to_zero_frames(&mut running.harness);
    let all = running
        .harness
        .centre(".sill-nc-head .ds-button")
        .expect("Clear All");
    running.harness.click(all);
    assert_eq!(
        running.commands_until(1),
        vec![NotificationsCommand::ClearHistory(Clear::All)]
    );
    running
        .feeds
        .notification_history
        .send_replace(History::default());
    running.until(|h| h.count(".sill-nc-empty") == 1);
    assert_settles_to_zero_frames(&mut running.harness);
}
