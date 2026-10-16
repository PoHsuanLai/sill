//! What the permission sheet draws and sends, on the harness over the fake capture service:
//! nothing until a question is put, or while the banners are on another output; with a question,
//! quire's `Alert` with the app's words and the two buttons the question names; each button, and
//! Escape, sending its answer once; nothing after the question is gone. All on the harness's
//! virtual clock; nothing here reaches a bus.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Driver, Harness, Input, Query, Viewport};
use shell_host::OutputId;
use sill_model::capture_pick::{
    AskId, CaptureCommand, CapturePicker, PermissionAsk, PermissionAsking, PermissionOutcome,
};
use sill_overlays::PermissionSheet;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput};

type Running = SurfaceHarness<PermissionSheet>;

const GRANT: &str = "#permission-grant";
const DENY: &str = "#permission-deny";

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn start(on: OutputId) -> Running {
    let link = BannerLink::new();
    link.set(BannerOutput::On(on));
    SurfaceHarness::<PermissionSheet>::setup(link, ())
        .viewport(VIEWPORT)
        .start()
}

fn question() -> CapturePicker {
    CapturePicker {
        permission: Some(PermissionAsking {
            id: AskId(7),
            ask: PermissionAsk::from_wire(
                "Notes",
                "Share your screen?",
                "Notes wants to see it",
                "Only while you share.",
                "Share",
                "Not Now",
            ),
        }),
        ..CapturePicker::default()
    }
}

fn sent(running: &mut Running) -> Vec<CaptureCommand> {
    std::iter::from_fn(|| running.feeds.capture_commands.try_recv().ok()).collect()
}

fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

fn up() -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    running.feeds.capture.send_replace(question());
    settle(&mut running.harness, |h| h.count(".ds-alert") == 1);
    running.harness.advance(Duration::from_millis(700));
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
fn until_a_question_is_put_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn a_question_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    running.feeds.capture.send_replace(question());
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn the_sheet_shows_the_apps_words_and_the_two_buttons_it_names() {
    let running = up();
    let html = running.harness.html();
    for word in [
        "Share your screen?",
        "Notes",
        "Notes wants to see it",
        "Only while you share.",
    ] {
        assert!(html.contains(word), "{word}: {html}");
    }
    assert_eq!(running.harness.count(GRANT), 1, "{html}");
    assert_eq!(running.harness.count(DENY), 1, "{html}");
    assert!(html.contains("Not Now"), "{html}");
}

#[test]
fn each_button_sends_its_answer_once() {
    for (button, outcome) in [
        (GRANT, PermissionOutcome::Grant),
        (DENY, PermissionOutcome::Deny),
    ] {
        let mut running = up();
        assert_eq!(
            sent(&mut running),
            Vec::new(),
            "{button}: nothing before the press"
        );
        click(&mut running, button);
        assert_eq!(
            sent(&mut running),
            vec![CaptureCommand::PermissionAnswer(AskId(7), outcome)],
            "{button}"
        );
    }
}

#[test]
fn escape_is_the_no_button() {
    let mut running = up();
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
        vec![CaptureCommand::PermissionAnswer(
            AskId(7),
            PermissionOutcome::Deny
        )]
    );
}

#[test]
fn the_sheet_leaves_once_the_question_is_gone() {
    let mut running = up();
    running.feeds.capture.send_replace(CapturePicker::default());
    settle(&mut running.harness, |h| h.count(".ds-alert") == 0);
    assert_eq!(running.harness.count(".ds-sheet"), 0);
}
