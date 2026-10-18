//! What the confirmation sheet draws and sends, on the harness over the fake intents service: nothing
//! while the queue is empty or the banners are elsewhere; with a question, quire's `Sheet` with the
//! action's own label, who asks, the arguments and reasons the router formatted, and the buttons;
//! the buttons disabled and Return ignored while it arms, Escape denying at any time; the offered
//! second answer ("Allow from the terminal: this action, until logout"); a destructive action's
//! Allow answering only after a hold; the next question entering with its own arming. All on the
//! harness's virtual clock; nothing here reaches a bus.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{FocusState, Harness, Input, Viewport};
use shell_host::OutputId;
use sill_model::intents::sheet::{Pending, Serial, SheetChoice, Sheets};
use sill_model::intents::{ConfirmId, Intents, IntentsCommand, IntentsLink};
use sill_overlays::ConfirmSheet;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput};

pub(crate) type Running = SurfaceHarness<ConfirmSheet>;

pub(crate) const ALLOW: &str = "#confirm-allow";
pub(crate) const DENY: &str = "#confirm-deny";
pub(crate) const OFFER: &str = "#confirm-offer";

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// The wire form of a `ConfirmRequest`: a terminal call to archive a thread.
pub(crate) fn wire(id: &str, offer: &str, gesture: &str, effect: &str) -> String {
    format!(
        r#"{{"id":"{id}","space":"desktop","actor":{{"kind":"cli"}},"app":"org.quire.Mail",
        "action":"Archive a thread","effect":"{effect}","count":1,"detail":{{"kind":"plain"}},
        "lines":[{{"label":"Thread","value":{{"kind":"plain","v":"Digest"}}}},
                 {{"label":"Subject","value":{{"kind":"quoted","v":{{"text":"Invoice 7","from":{{"kind":"mail"}}}}}}}}],
        "why":[{{"kind":"from_terminal"}}],"taint":{{"kind":"clean"}},"offer":"{offer}",
        "gesture":"{gesture}","anchor":{{"kind":"centre"}},"expires":120}}"#
    )
}

fn pending(serial: u64, id: &str, offer: &str, gesture: &str, effect: &str) -> Pending {
    let request =
        serde_json::from_str(&wire(id, offer, gesture, effect)).unwrap_or_else(|e| panic!("{e}"));
    Pending {
        serial: Serial(serial),
        request,
    }
}

/// The sheet, hidden, with the banners on `on` (the test output is `OutputId(1)`).
fn start(on: OutputId) -> Running {
    let link = BannerLink::new();
    link.set(BannerOutput::On(on));
    SurfaceHarness::<ConfirmSheet>::setup(link, ())
        .viewport(VIEWPORT)
        .start()
}

pub(crate) fn ask(running: &mut Running, queue: Vec<Pending>) {
    running.feeds.intents.send_replace(Intents {
        link: IntentsLink::Serving,
        sheets: Sheets { queue },
        ..Intents::default()
    });
}

/// What the sheet has sent the service since the last call.
pub(crate) fn sent(running: &mut Running) -> Vec<IntentsCommand> {
    std::iter::from_fn(|| running.feeds.intents_commands.try_recv().ok()).collect()
}

pub(crate) fn answer(id: &str, choice: SheetChoice) -> IntentsCommand {
    IntentsCommand::Answer {
        id: ConfirmId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        choice,
    }
}

fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The sheet showing `queue`, once it has entered but still arming (200 ms in).
pub(crate) fn arming(queue: Vec<Pending>) -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    ask(&mut running, queue);
    settle(&mut running.harness, |h| h.count(".sill-confirm") == 1);
    running
}

/// The sheet once it is armed (past the 500 ms arming).
pub(crate) fn armed(queue: Vec<Pending>) -> Running {
    let mut running = arming(queue);
    running.harness.advance(Duration::from_millis(700));
    running
}

fn centre(running: &Running, selector: &str) -> Point {
    running
        .harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not laid out: {}", running.harness.html()))
}

pub(crate) fn click(running: &mut Running, selector: &str) {
    let at = centre(running, selector);
    running.harness.send(Input::click(at));
    running.harness.advance(Duration::from_millis(40));
}

#[test]
fn with_no_question_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".sill-confirm"), 0);
}

#[test]
fn a_question_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    ask(
        &mut running,
        vec![pending(1, "c-1", "once_only", "press", "undoable_write")],
    );
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".sill-confirm"), 0);
}

#[test]
fn a_question_draws_the_routers_words_on_a_narrow_centred_sheet() {
    let running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.attr(".ds-sheet", "data-width").as_deref(), Some("narrow"));
    assert_eq!(
        h.attr(".ds-sheet", "data-attach").as_deref(),
        Some("centre")
    );
    assert_eq!(
        h.count(".ds-sheet#confirm-panel"),
        1,
        "the sheet carries the blur region's id"
    );
    assert_eq!(h.count(".ds-scrim"), 0, "a sheet dims nothing");
    assert_eq!(
        h.text_of(".sill-confirm-title").as_deref(),
        Some("Archive a thread?")
    );
    assert_eq!(
        h.text_of(".sill-confirm-byline").as_deref(),
        Some("The terminal wants to do this in Mail.")
    );
    let html = h.html();
    for expected in ["Thread", "Digest", "Subject", "Invoice 7", "from a message"] {
        assert!(html.contains(expected), "{expected:?} is on the sheet");
    }
    assert_eq!(
        h.count(".sill-confirm-note"),
        2,
        "the reason and the effect"
    );
    let sheet = h.rect(".ds-sheet").expect("the sheet is laid out"); // test-only
    let middle = sheet.origin.x.0 + sheet.size.width.0 / 2.0;
    assert!((middle - 640.0).abs() < 1.0, "centred: {sheet:?}");
    assert!(sheet.size.width.0 <= 340.5, "narrow: {sheet:?}");
}

#[test]
fn the_buttons_are_disabled_while_it_arms_and_a_press_then_answers_nothing() {
    let mut running = arming(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        running.harness.attr(".sill-confirm", "data-arm").as_deref(),
        Some("arming")
    );
    for button in [ALLOW, DENY] {
        assert_eq!(
            running.harness.attr(button, "data-availability").as_deref(),
            Some("disabled"),
            "{button}"
        );
    }
    click(&mut running, ALLOW);
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(40));
    assert!(
        sent(&mut running).is_empty(),
        "nothing answers while it arms"
    );
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(
        running.harness.attr(".sill-confirm", "data-arm").as_deref(),
        Some("armed")
    );
}

#[test]
fn escape_denies_even_while_it_arms() {
    let mut running = arming(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(sent(&mut running), vec![answer("c-1", SheetChoice::Deny)]);
}

#[test]
fn escape_denies_once_armed_too() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(sent(&mut running), vec![answer("c-1", SheetChoice::Deny)]);
}

#[test]
fn once_armed_allow_and_dont_allow_answer_and_only_once() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    assert_eq!(
        running.harness.count(".sill-confirm-offer"),
        0,
        "nothing else is offered"
    );
    assert_eq!(running.harness.text_of(ALLOW).as_deref(), Some("Allow"));
    assert_eq!(
        running.harness.text_of(DENY).as_deref(),
        Some("Don't Allow")
    );
    click(&mut running, ALLOW);
    click(&mut running, ALLOW);
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowOnce)],
        "one answer per question, the later presses count for nothing"
    );
    let mut running = armed(vec![pending(
        1,
        "c-2",
        "once_only",
        "press",
        "undoable_write",
    )]);
    click(&mut running, DENY);
    assert_eq!(sent(&mut running), vec![answer("c-2", SheetChoice::Deny)]);
}

#[test]
fn return_presses_allow_once_armed() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowOnce)]
    );
}

#[test]
fn the_terminal_offer_is_a_button_that_answers_allowed_from_the_terminal() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_or_from_terminal",
        "press",
        "undoable_write",
    )]);
    assert_eq!(running.harness.count(".sill-confirm-offer"), 1);
    assert_eq!(
        running.harness.text_of(OFFER).as_deref(),
        Some("Allow from the terminal: this action, until logout")
    );
    click(&mut running, OFFER);
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowFromTerminal)]
    );
}

#[test]
fn always_is_offered_only_when_the_router_offers_it() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_or_always",
        "press",
        "undoable_write",
    )]);
    assert_eq!(
        running.harness.text_of(OFFER).as_deref(),
        Some("Always Allow")
    );
    click(&mut running, OFFER);
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowAlways)]
    );
    let running = armed(vec![pending(
        1,
        "c-2",
        "once_only",
        "press",
        "undoable_write",
    )]);
    assert!(!running.harness.html().contains("Always Allow"));
    assert!(!running.harness.html().contains("from the terminal"));
}

#[test]
fn a_destructive_action_asks_for_a_hold_not_a_press() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "hold_to_confirm",
        "destructive",
    )]);
    assert_eq!(
        running
            .harness
            .attr(".sill-confirm", "data-gesture")
            .as_deref(),
        Some("hold")
    );
    assert_eq!(
        running.harness.text_of(ALLOW).as_deref(),
        Some("Hold to Allow")
    );
    let at = centre(&running, ALLOW);
    // A click is not a hold.
    running.harness.send(Input::click(at));
    running.harness.advance(Duration::from_millis(1500));
    assert!(sent(&mut running).is_empty());
    // Let go early: abandoned.
    running.harness.send(Input::pointer_down(at));
    running.harness.advance(Duration::from_millis(600));
    assert_eq!(
        running
            .harness
            .attr(".sill-confirm-hold", "data-holding")
            .as_deref(),
        Some("true")
    );
    assert_eq!(
        running.harness.text_of(ALLOW).as_deref(),
        Some("Keep holding\u{2026}")
    );
    running.harness.send(Input::pointer_up(at));
    running.harness.advance(Duration::from_millis(1500));
    assert!(
        sent(&mut running).is_empty(),
        "released before the hold was done"
    );
    // Hold it through.
    running.harness.send(Input::pointer_down(at));
    running.harness.advance(Duration::from_millis(1100));
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowOnce)]
    );
}

#[test]
fn a_hold_does_not_start_while_it_arms_and_return_never_allows_a_destructive_action() {
    let mut running = arming(vec![pending(
        1,
        "c-1",
        "once_only",
        "hold_to_confirm",
        "destructive",
    )]);
    let at = centre(&running, ALLOW);
    running.harness.send(Input::pointer_down(at));
    running.harness.advance(Duration::from_millis(300));
    running.harness.send(Input::pointer_up(at));
    running.harness.advance(Duration::from_millis(1500));
    assert!(sent(&mut running).is_empty());
    // Armed: the keyboard is on Don't Allow, which Space presses and Return does not; nothing
    // the keyboard does in one stroke allows it.
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(40));
    assert!(sent(&mut running).is_empty(), "Return answered nothing");
    running.harness.send(Input::key(ShortcutKey::Char(' ')));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(sent(&mut running), vec![answer("c-1", SheetChoice::Deny)]);
}

#[test]
fn a_key_tap_on_the_hold_button_is_not_a_hold() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "hold_to_confirm",
        "destructive",
    )]);
    // The keyboard is on Don't Allow; Tab reaches the hold button, whose Return or Space begins
    // the hold and whose key release ends it (the harness taps: down and up at once).
    running.harness.send(Input::key(ShortcutKey::Tab));
    running.harness.advance(Duration::from_millis(40));
    running.harness.send(Input::key(ShortcutKey::Char(' ')));
    running.harness.advance(Duration::from_millis(2000));
    assert!(sent(&mut running).is_empty(), "a tap is not a hold");
    assert_eq!(
        running
            .harness
            .attr(".sill-confirm-hold", "data-holding")
            .as_deref(),
        Some("false")
    );
    // The abandoned hold leaves the keyboard where the person put it, not back on Don't Allow.
    assert_eq!(running.harness.focus_of(ALLOW), FocusState::Focused);
}

#[test]
fn the_next_question_enters_with_its_own_arming() {
    let mut running = armed(vec![
        pending(1, "c-1", "once_only", "press", "undoable_write"),
        pending(2, "c-2", "once_only", "press", "undoable_write"),
    ]);
    assert_eq!(
        running.harness.text_of(".sill-confirm-waiting").as_deref(),
        Some("1 more waiting")
    );
    click(&mut running, ALLOW);
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowOnce)]
    );
    // The service drops the answered question; the next is on screen, arming again.
    ask(
        &mut running,
        vec![pending(2, "c-2", "once_only", "press", "undoable_write")],
    );
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".sill-confirm-waiting"), 0);
    assert_eq!(
        running.harness.attr(".sill-confirm", "data-arm").as_deref(),
        Some("arming"),
        "a new question never inherits the last one's armed buttons"
    );
    click(&mut running, ALLOW);
    assert!(sent(&mut running).is_empty());
    running.harness.advance(Duration::from_millis(700));
    click(&mut running, ALLOW);
    assert_eq!(
        sent(&mut running),
        vec![answer("c-2", SheetChoice::AllowOnce)]
    );
}

#[test]
fn a_question_that_goes_plays_the_sheet_out_then_draws_nothing() {
    let mut running = armed(vec![pending(
        1,
        "c-1",
        "once_only",
        "press",
        "undoable_write",
    )]);
    ask(&mut running, Vec::new());
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.count(".sill-confirm"),
        1,
        "still playing its exit"
    );
    settle(&mut running.harness, |h| h.count(".sill-confirm") == 0);
    assert_eq!(running.harness.count(".ds-sheet"), 0);
}
