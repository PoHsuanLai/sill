//! The banner column in a Blitz harness, on the virtual clock and the fake services, drawn by
//! quire's `BannerStack` and `NotificationCard`: a banner slides in and holds for
//! `notifications.banner_hold_ms`, then says it timed out and plays out; hovering it pauses the
//! hold; a press on the card invokes `default`; the close button dismisses it; a drag past
//! quire's threshold swipes it away (`SwipeAway`), a short one springs back; a link in the body
//! keeps its press; one app's banners group into one card with a count and two layers; a card
//! the service dropped leaves silently; another output's column draws nothing; and the keyboard
//! ([`keys`]) is asked for and never taken.

use std::time::Duration;

use ds::Point;
use ds_settings::Ms;
use shell_host::OutputId;
use sill_model::notifications::{
    Action, ActionKey, Activation, AppRef, BannerLife, Body, Expiry, Hints, IconRef, LiveBanner,
    Notification, NotificationId, Notifications, NotificationsCommand, UnixMs, parse_body,
};
use sill_notify_ui::Banners;
use sill_settings::{NotificationsSettings, ShellFile};
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput};

mod keys;

type Running = SurfaceHarness<Banners>;

/// The column on `file`'s settings, shown on the test output, nothing posted yet.
fn start(file: ShellFile) -> Running {
    let link = BannerLink::new();
    link.set(BannerOutput::On(fake::output().id));
    SurfaceHarness::<Banners>::setup(link, ())
        .settings(file)
        .start()
}

/// Publish `banners` (newest first) as the service would, taken at this instant (the next
/// `advance` runs the woken column first).
fn show(running: &mut Running, banners: Vec<LiveBanner>) {
    running.feeds.notifications.send_modify(|n| {
        n.live = Notifications {
            banners,
            ..Notifications::default()
        }
    });
}

/// Every command sent since the last call.
fn commands(running: &mut Running) -> Vec<NotificationsCommand> {
    std::iter::from_fn(|| running.feeds.notification_commands.try_recv().ok()).collect()
}

/// How many cards quire's stack draws (listed or playing out).
fn cards(running: &Running) -> usize {
    running.harness.count(".ds-banner")
}

/// How many of them are playing their exit.
fn leaving(running: &Running) -> usize {
    running.harness.count(".ds-banner[*|data-presence=leaving]")
}

/// The first card's `data-hover` (`on` under the pointer).
fn hover(running: &Running) -> Option<String> {
    running.harness.attr(".ds-notification", "data-hover")
}

/// The first card's plate: where a press opens it.
fn plate(running: &Running) -> Point {
    running
        .harness
        .centre(".ds-notification-plate")
        .expect("a card") // test-only
}

/// Advance in 20 ms steps until `done`, at most `limit` of the harness's clock; whether it
/// happened.
fn until(running: &mut Running, limit: Duration, done: impl Fn(&Running) -> bool) -> Done {
    let end = running.harness.now() + limit;
    while running.harness.now() < end {
        running.harness.advance(Duration::from_millis(20));
        if done(running) {
            return Done::Yes;
        }
    }
    Done::No
}

/// Whether a wait ended in what it waited for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Done {
    Yes,
    No,
}

/// A notification `id` from `app` with `summary`, `body` and `actions` (`key=label`).
fn note(id: u32, app: &str, summary: &str, body: &str, actions: &[(&str, &str)]) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(sill_ui_kit::notification_card::wall_now().0),
        app: AppRef {
            name: app.to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: Body::plain(body),
        actions: actions
            .iter()
            .map(|(key, label)| Action {
                key: ActionKey((*key).to_owned()),
                label: (*label).to_owned(),
            })
            .collect(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

/// `note` as a banner that expires.
fn live(note: Notification) -> LiveBanner {
    LiveBanner {
        note,
        life: BannerLife::Expires,
    }
}
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
    show(
        &mut running,
        vec![live(note(1, "mail", "Hello", "A body", actions))],
    );
    assert_eq!(
        until(&mut running, Duration::from_millis(900), |r| cards(r) == 1),
        Done::Yes,
        "the card slides in"
    );
    running.harness.advance(Duration::from_millis(400));
    running
}

fn right(from: Point, dx: f32) -> Point {
    Point {
        x: ds::Px(from.x.0 + dx),
        y: from.y,
    }
}

/// A drag of `dx` from `from` in four moves, `step` apart.
fn drag(running: &mut Running, from: Point, dx: f32, step: Duration) {
    running.harness.pointer_down(from);
    for n in 1..=4u8 {
        running.harness.advance(step);
        running
            .harness
            .pointer_move(right(from, dx * f32::from(n) / 4.0));
    }
    running.harness.advance(step);
    running.harness.pointer_up(right(from, dx));
}

#[test]
fn a_banner_slides_in_holds_and_times_out() {
    let mut running = start(file());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(cards(&running), 0, "nothing before a banner");
    show(
        &mut running,
        vec![live(note(1, "mail", "Hello", "A body", &[]))],
    );
    let shown = running.harness.now();
    running.harness.advance(Duration::from_millis(20));
    assert_eq!(cards(&running), 1, "quire's stack slides it in");
    let h = &running.harness;
    assert_eq!(
        h.text_of(".ds-notification-summary").as_deref(),
        Some("Hello")
    );
    assert_eq!(h.text_of(".ds-notification-age").as_deref(), Some("now"));
    assert_eq!(
        h.count("#toast-1.ds-notification-plate"),
        1,
        "the plate's id"
    );
    assert_eq!(h.count("#banner-1"), 1, "the whole card's id");
    // The hold starts once the card has risen, a `FRAME_SLACK` after it was shown.
    let leaves = shown + ds::FRAME_SLACK + Duration::from_millis(HOLD_MS);
    let before = leaves - running.harness.now() - Duration::from_millis(1);
    running.harness.advance(before);
    assert_eq!(
        leaving(&running),
        0,
        "not a millisecond before the hold's end"
    );
    assert!(commands(&mut running).is_empty());
    running.harness.advance(Duration::from_millis(1));
    assert_eq!(
        leaving(&running),
        1,
        "it leaves by itself at the hold's end"
    );
    assert_eq!(
        commands(&mut running),
        [NotificationsCommand::BannerTimedOut(NotificationId(1))]
    );
    show(&mut running, Vec::new());
    assert_eq!(
        until(&mut running, Duration::from_millis(1000), |r| cards(r) == 0),
        Done::Yes,
        "gone once its exit has played"
    );
}

#[test]
fn hovering_opens_the_card_and_pauses_the_hold() {
    let mut running = one(&[("reply", "Reply")]);
    let plate = plate(&running);
    running.harness.pointer_move(plate);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(hover(&running).as_deref(), Some("on"));
    assert_eq!(
        running.harness.count(".ds-notification-close"),
        1,
        "the close button"
    );
    assert_eq!(
        running.harness.count(".ds-notification-actions .ds-button"),
        1,
        "Reply"
    );
    running
        .harness
        .advance(Duration::from_millis(HOLD_MS + 500));
    assert_eq!(leaving(&running), 0, "paused while hovered");
    assert!(commands(&mut running).is_empty());
    running.harness.pointer_move(Point {
        x: ds::Px(2.0),
        y: ds::Px(690.0),
    });
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(hover(&running).as_deref(), Some("off"));
    assert_eq!(
        until(&mut running, Duration::from_millis(3000), |r| leaving(r)
            == 1),
        Done::Yes,
        "the hold resumes once the pointer has gone"
    );
}

#[test]
fn a_press_on_the_card_invokes_default() {
    let mut running = one(&[("default", "Open")]);
    let plate = plate(&running);
    running.harness.click(plate);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        commands(&mut running),
        [NotificationsCommand::InvokeAction {
            id: NotificationId(1),
            action: ActionKey::default_action(),
            token: Activation::NoToken,
        }]
    );
    assert_eq!(leaving(&running), 1, "the card leaves");
}

#[test]
fn the_close_button_dismisses() {
    let mut running = one(&[]);
    let plate = plate(&running);
    running.harness.pointer_move(plate);
    running.harness.advance(Duration::from_millis(300));
    let close = running
        .harness
        .centre(".ds-notification-close")
        .expect("a close button"); // test-only
    running.harness.click(close);
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(
        commands(&mut running),
        [NotificationsCommand::Dismiss(NotificationId(1))]
    );
    assert_eq!(leaving(&running), 1);
}

#[test]
fn a_long_drag_swipes_away_and_a_short_one_springs_back() {
    let mut running = one(&[("default", "Open")]);
    let from = plate(&running);
    drag(&mut running, from, 40.0, Duration::from_millis(100));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(leaving(&running), 0, "a short, slow drag springs back");
    assert!(
        commands(&mut running).is_empty(),
        "and its click is swallowed"
    );
    drag(&mut running, from, 100.0, Duration::from_millis(20));
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(leaving(&running), 1, "flies out");
    assert_eq!(
        commands(&mut running),
        [NotificationsCommand::SwipeAway(NotificationId(1))],
        "swiped away: the service keeps it in the center (notifications.swipe)"
    );
}

#[test]
fn a_link_in_the_body_keeps_its_press() {
    let mut running = start(file());
    let mut linked = note(1, "mail", "Build", "", &[("default", "Open")]);
    linked.body = parse_body("<i>See</i> <a href=\"https://ci.example/42\">the log</a>");
    show(&mut running, vec![live(linked)]);
    running.harness.advance(Duration::from_millis(400));
    let h = &running.harness;
    assert_eq!(h.count(".ds-run-link"), 1);
    assert_eq!(
        h.attr(".ds-run[*|data-tone=italic]", "data-tone")
            .as_deref(),
        Some("italic")
    );
    let link = h.centre(".ds-run-link").expect("the link"); // test-only
    running.harness.click(link);
    running.harness.advance(Duration::from_millis(60));
    assert!(commands(&mut running).is_empty(), "the card is not opened");
    assert_eq!(leaving(&running), 0);
}

#[test]
fn one_apps_banners_group_into_one_card_and_five_apps_make_three() {
    let mut running = start(file());
    show(
        &mut running,
        vec![
            live(note(3, "chat", "Third", "", &[])),
            live(note(2, "chat", "Second", "", &[])),
            live(note(1, "chat", "First", "", &[])),
        ],
    );
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(cards(&running), 1);
    let h = &running.harness;
    assert_eq!(
        h.text_of(".ds-notification-summary").as_deref(),
        Some("Third"),
        "the newest"
    );
    assert_eq!(h.text_of(".ds-notification-count").as_deref(), Some("3"));
    assert_eq!(h.count(".ds-notification-layer"), 2, "two offset layers");
    let apps = ["a", "b", "c", "d", "e"];
    show(
        &mut running,
        apps.iter()
            .enumerate()
            .rev()
            .map(|(i, app)| live(note(10 + i as u32, app, app, "", &[])))
            .collect(),
    );
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        cards(&running),
        4,
        "three new cards, the old one sliding out"
    );
    assert_eq!(leaving(&running), 1);
    let summaries: Vec<String> = (0..4)
        .filter_map(|i| {
            running.harness.text_of(&format!(
                ".ds-banner:nth-child({}) .ds-notification-summary",
                i + 1
            ))
        })
        .collect();
    assert_eq!(
        summaries,
        ["Third", "e", "d", "c"],
        "the leaving card keeps its place until it has gone; the new ones newest first"
    );
}

#[test]
fn a_dropped_banner_leaves_silently_and_another_output_draws_nothing() {
    let mut running = one(&[]);
    show(&mut running, Vec::new());
    running.harness.advance(Duration::from_millis(60));
    assert_eq!(leaving(&running), 1);
    assert!(commands(&mut running).is_empty());
    assert_eq!(
        until(&mut running, Duration::from_millis(1000), |r| cards(r) == 0),
        Done::Yes
    );
    running.link.set(BannerOutput::On(OutputId(99)));
    show(
        &mut running,
        vec![live(note(2, "mail", "Elsewhere", "", &[]))],
    );
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(cards(&running), 0);
}

#[test]
fn the_entry_edge_follows_the_setting() {
    for (edge, slug) in [
        (sill_settings::BannerEntry::FromRight, "right"),
        (sill_settings::BannerEntry::FromBelow, "below"),
    ] {
        let mut settings = file();
        settings.notifications.banner_entry_direction = edge;
        let mut running = start(settings);
        show(
            &mut running,
            vec![live(note(1, "mail", "Hello", "A body", &[]))],
        );
        running.harness.advance(Duration::from_millis(20));
        assert_eq!(
            running
                .harness
                .attr(".ds-banner-stack", "data-entry")
                .as_deref(),
            Some(slug),
            "{edge:?}"
        );
    }
}

#[test]
fn the_stylesheet_names_no_motion_of_its_own() {
    for motion in ["@keyframes", "transition", "animation"] {
        assert!(
            !sill_notify_ui::banners::BANNERS_CSS.contains(motion),
            "{motion}: the stack's motion is quire's"
        );
    }
}
