//! What the polkit prompt draws and sends: nothing while the agent has no request or the
//! banners are elsewhere; with one, quire's `PolkitPrompt` on a narrow centred sheet over the
//! full-output modal scrim (the action's sentence, the identity asked for, the details); Enter
//! sends the typed text as the answer and Escape a dismissal; a wrong try shakes; a request
//! that goes plays the sheet out and draws nothing once it has settled.

mod polkit_prompt_support;

use std::time::Duration;

use ds::Key;
use polkit_prompt_support::{HEIGHT, WIDTH, asking, fake, request, settle, shown, start};
use shell_host::OutputId;
use sill_services::polkit_agent::{Cookie, LastTry, PolkitCommand, PromptPhase};

#[test]
fn with_no_request_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".ds-scrim"), 0);
    assert_eq!(running.harness.count(".ds-polkit"), 0);
}

#[test]
fn a_request_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    running.show(request("c1", asking(), LastTry::None));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".ds-polkit"), 0);
}

#[test]
fn a_request_draws_quires_prompt_on_a_narrow_sheet_over_the_scrim() {
    let running = shown(request("c1", asking(), LastTry::None));
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.attr(".ds-sheet", "data-width").as_deref(), Some("narrow"));
    assert_eq!(
        h.count(&format!(
            ".ds-sheet#{}",
            sill_surfaces::surfaces::polkit_prompt::PANEL_ID
        )),
        1,
        "the sheet carries the blur region's id"
    );
    assert_eq!(
        h.attr(".ds-sheet", "data-placement").as_deref(),
        Some("centre")
    );
    assert_eq!(
        h.attr(".ds-scrim", "data-strength").as_deref(),
        Some("modal")
    );
    assert_eq!(h.attr(".ds-polkit", "data-state").as_deref(), Some("idle"));
    assert_eq!(
        h.text_of(".ds-polkit-title").as_deref(),
        Some("Authentication Required")
    );
    assert_eq!(
        h.text_of(".ds-polkit-action").as_deref(),
        Some("Authentication is required to start 'sshd.service'.")
    );
    assert_eq!(h.text_of(".ds-polkit-user").as_deref(), Some("ada"));
    assert_eq!(h.count(".ds-polkit-details"), 1);
    let scrim = h.rect(".ds-scrim").expect("the scrim is laid out"); // test-only
    assert_eq!(
        (scrim.size.width.0, scrim.size.height.0),
        (WIDTH, HEIGHT),
        "the whole output dims"
    );
    let sheet = h.rect(".ds-sheet").expect("the sheet is laid out"); // test-only
    let middle = sheet.origin.x.0 + sheet.size.width.0 / 2.0;
    assert!((middle - WIDTH / 2.0).abs() < 1.0, "centred: {sheet:?}");
    assert!(sheet.size.width.0 <= 340.5, "narrow: {sheet:?}");
}

#[test]
fn enter_answers_with_what_was_typed_and_escape_dismisses() {
    let mut running = shown(request("c1", asking(), LastTry::None));
    for c in "sill-fake".chars() {
        running.harness.key(Key::Char(c));
    }
    running.harness.key(Key::Enter);
    running.harness.advance(Duration::from_millis(40));
    let sent = running.sent();
    let [PolkitCommand::Answer { cookie, secret }] = sent.as_slice() else {
        panic!("one answer: {sent:?}");
    };
    assert_eq!(cookie, &Cookie("c1".to_owned()));
    assert_eq!(secret.expose(), "sill-fake");
    assert_eq!(format!("{secret:?}"), "Secret(..)", "never printed");
    assert!(
        !running.harness.html().contains("sill-fake"),
        "the secret is not in the markup"
    );
    running.harness.key(Key::Escape);
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.sent(),
        vec![PolkitCommand::Dismiss(Cookie("c1".to_owned()))]
    );
}

#[test]
fn checking_closes_the_field_and_a_wrong_try_shakes() {
    let mut running = shown(request("c1", PromptPhase::Checking, LastTry::None));
    assert_eq!(
        running.harness.attr(".ds-polkit", "data-state").as_deref(),
        Some("checking")
    );
    running.show(request("c1", asking(), LastTry::Wrong));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.attr(".ds-polkit", "data-state").as_deref(),
        Some("wrong")
    );
    assert_eq!(
        running.harness.count(".ds-polkit-field"),
        1,
        "the field stays"
    );
}

#[test]
fn a_request_that_goes_plays_the_sheet_out_then_draws_nothing() {
    let mut running = shown(request("c1", asking(), LastTry::None));
    running.clear();
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running
            .harness
            .attr(".ds-sheet", "data-presence")
            .as_deref(),
        Some("leaving")
    );
    settle(&mut running.harness, |h| h.count(".ds-sheet") == 0);
    assert_eq!(running.harness.count(".ds-polkit"), 0);
    running.show(request("c2", asking(), LastTry::None));
    settle(&mut running.harness, |h| h.count(".ds-polkit") == 1);
    assert_eq!(
        running.harness.count(".ds-polkit"),
        1,
        "the next one enters"
    );
}
