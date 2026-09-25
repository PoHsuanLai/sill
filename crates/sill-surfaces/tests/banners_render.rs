//! The banner column in a Blitz harness, on the wall clock and the fake services: a banner
//! slides in and holds for `notifications.banner_hold_ms`, then says it timed out; hovering it
//! expands it and pauses the hold; a click on the body invokes `default`; the close button
//! dismisses it, a drag past the threshold takes the banner away (kept in the center), a short
//! drag springs back; one app's banners group into
//! one card with a count and two layers; a card the service dropped slides out silently; and
//! another output's column draws nothing.

mod banners_support;

use std::time::Duration;

use banners_support::{Done, Running, live, note, start};
use ds::Point;
use ds_settings::Ms;
use shell_host::OutputId;
use sill_services::{ActionKey, Activation, NotificationId, NotificationsCommand};
use sill_settings::{NotificationsSettings, ShellFile};
use sill_surfaces::surfaces::banners::BannerOutput;

const HOLD_MS: u64 = 1000;

fn file() -> ShellFile {
    ShellFile {
        notifications: NotificationsSettings {
            banner_hold_ms: Ms(HOLD_MS as u16),
            ..NotificationsSettings::default()
        },
        ..ShellFile::default()
    }
}

/// One banner from `mail`, shown and entered.
fn one(actions: &[(&str, &str)]) -> Running {
    let mut running = start(file());
    running.show(vec![live(note(1, "mail", "Hello", "A body", actions))]);
    assert_eq!(
        running.until(Duration::from_millis(900), |r| r.pose().as_deref() == Some("rest")),
        Done::Yes,
        "the card slides in"
    );
    // Let the entrance settle (the hold starts once it has).
    running.harness.advance(Duration::from_millis(500));
    running
}

fn card_centre(running: &Running) -> Point {
    running.harness.centre(".sill-card").expect("a card") // test-only
}

#[test]
fn a_banner_slides_in_holds_and_times_out() {
    let mut running = start(file());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.cards(), 0, "nothing before a banner");
    running.show(vec![live(note(1, "mail", "Hello", "A body", &[]))]);
    running.harness.advance(Duration::from_millis(1));
    assert_eq!(running.pose().as_deref(), Some("offstage"), "mounted off the edge");
    assert_eq!(
        running.harness.text_of(".sill-card-summary").as_deref(),
        Some("Hello")
    );
    assert_eq!(running.harness.text_of(".sill-card-age").as_deref(), Some("now"));
    let shown = std::time::Instant::now();
    assert_eq!(
        running.until(Duration::from_millis(900), |r| r.pose().as_deref() == Some("rest")),
        Done::Yes,
        "it slides in"
    );
    assert_eq!(
        running.until(Duration::from_millis(3000), |r| r.pose().as_deref() == Some("offstage")),
        Done::Yes,
        "it leaves by itself"
    );
    let held = shown.elapsed();
    assert!(
        held >= Duration::from_millis(HOLD_MS),
        "not before the hold: {held:?}"
    );
    assert_eq!(
        running.commands(),
        [NotificationsCommand::BannerTimedOut(NotificationId(1))]
    );
    running.show(Vec::new());
    assert_eq!(
        running.until(Duration::from_millis(1000), |r| r.cards() == 0),
        Done::Yes,
        "gone once its exit has played"
    );
}

#[test]
fn hovering_expands_and_pauses_the_hold() {
    let mut running = one(&[("reply", "Reply")]);
    let centre = card_centre(&running);
    running.harness.pointer_move(centre);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.expansion().as_deref(), Some("expanded"));
    assert_eq!(running.harness.count(".sill-card-close"), 1, "the close button");
    assert_eq!(running.harness.count(".sill-card-actions .ds-button"), 1, "Reply");
    running.harness.advance(Duration::from_millis(HOLD_MS + 500));
    assert_eq!(running.pose().as_deref(), Some("rest"), "paused while hovered");
    assert!(running.commands().is_empty());
    running.harness.pointer_move(Point {
        x: ds::Px(2.0),
        y: ds::Px(690.0),
    });
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.expansion().as_deref(), Some("collapsed"));
}

#[test]
fn a_click_on_the_body_invokes_default() {
    let mut running = one(&[("default", "Open")]);
    let centre = card_centre(&running);
    running.harness.click(centre);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        running.commands(),
        [NotificationsCommand::InvokeAction {
            id: NotificationId(1),
            action: ActionKey::default_action(),
            token: Activation::NoToken,
        }]
    );
    assert_eq!(running.pose().as_deref(), Some("offstage"), "the card leaves");
}

#[test]
fn the_close_button_dismisses() {
    let mut running = one(&[]);
    running.harness.pointer_move(card_centre(&running));
    running.harness.advance(Duration::from_millis(60));
    let close = running
        .harness
        .centre(".sill-card-close .ds-icon-button")
        .expect("a close button"); // test-only
    running.harness.click(close);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(
        running.commands(),
        [NotificationsCommand::Dismiss(NotificationId(1))]
    );
}

#[test]
fn a_long_drag_dismisses_and_a_short_one_springs_back() {
    let mut running = one(&[("default", "Open")]);
    let from = card_centre(&running);
    let right = |dx: f32| Point {
        x: ds::Px(from.x.0 + dx),
        y: from.y,
    };
    running.harness.pointer_down(from);
    running.harness.pointer_move(right(10.0));
    running.harness.pointer_move(right(30.0));
    running.harness.advance(Duration::from_millis(30));
    assert_eq!(running.pose().as_deref(), Some("follow"), "follows the pointer");
    running.harness.advance(Duration::from_millis(400));
    running.harness.pointer_up(right(30.0));
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.pose().as_deref(), Some("rest"), "a short, slow drag springs back");
    assert!(running.commands().is_empty(), "and its click is swallowed");
    running.harness.pointer_down(from);
    running.harness.pointer_move(right(50.0));
    running.harness.pointer_move(right(95.0));
    running.harness.pointer_up(right(95.0));
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.pose().as_deref(), Some("offstage"), "flies out");
    assert_eq!(
        running.commands(),
        [NotificationsCommand::BannerTimedOut(NotificationId(1))],
        "swiped away, kept in the center (notifications.swipe's default, as macOS)"
    );
}

#[test]
fn one_apps_banners_group_into_one_card_and_five_apps_make_three() {
    let mut running = start(file());
    running.show(vec![
        live(note(3, "chat", "Third", "", &[])),
        live(note(2, "chat", "Second", "", &[])),
        live(note(1, "chat", "First", "", &[])),
    ]);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.cards(), 1);
    assert_eq!(
        running.harness.text_of(".sill-card-summary").as_deref(),
        Some("Third"),
        "the newest"
    );
    assert_eq!(running.harness.text_of(".ds-count").as_deref(), Some("3"));
    assert_eq!(running.harness.count(".sill-card-layer"), 2, "two offset layers");
    let apps = ["a", "b", "c", "d", "e"];
    running.show(
        apps.iter()
            .enumerate()
            .rev()
            .map(|(i, app)| live(note(10 + i as u32, app, app, "", &[])))
            .collect(),
    );
    running.harness.advance(Duration::from_millis(100));
    let summaries: Vec<String> = (0..running.cards())
        .filter_map(|i| {
            running
                .harness
                .text_of(&format!(".sill-banner:nth-child({}) .sill-card-summary", i + 1))
        })
        .collect();
    assert_eq!(running.cards(), 4, "three new cards, the old one sliding out");
    assert!(summaries.starts_with(&["e".to_owned(), "d".to_owned(), "c".to_owned()]), "{summaries:?}");
}

#[test]
fn a_dropped_banner_leaves_silently_and_another_output_draws_nothing() {
    let mut running = one(&[]);
    running.show(Vec::new());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(running.pose().as_deref(), Some("offstage"));
    assert!(running.commands().is_empty());
    running.link.set(BannerOutput::On(OutputId(99)));
    running.show(vec![live(note(2, "mail", "Elsewhere", "", &[]))]);
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(running.cards(), 0);
}
