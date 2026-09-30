//! What the polkit prompt draws and sends: nothing while the agent has no request or the
//! banners are elsewhere; with one, quire's `PolkitPrompt` on a narrow centred sheet over the
//! full-output modal scrim (the action's sentence, the identity asked for, the details); Enter
//! sends the typed text as the answer and Escape a dismissal; a wrong try shakes; a request
//! that goes plays the sheet out and draws nothing once it has settled.

use std::time::Duration;

use ds::ShortcutKey;
use ds_harness::{Driver, Harness, Input, Query, Viewport};
use shell_host::OutputId;
use sill_model::auth::UserName;
use sill_model::polkit_agent::{
    ActionId, Cookie, Echo, LastTry, Polkit, PolkitCommand, PromptPhase, PromptView, Registration,
};
use sill_overlays::PolkitPrompt;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput};

type Running = SurfaceHarness<PolkitPrompt>;

const WIDTH: f32 = 1280.0;

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// The prompt, hidden, with the banners on `on` (the test output is `OutputId(1)`).
fn start(on: OutputId) -> Running {
    let link = BannerLink::new();
    link.set(BannerOutput::On(on));
    SurfaceHarness::<PolkitPrompt>::setup(link, ())
        .viewport(VIEWPORT)
        .start()
}

/// Show `view` as the agent's current request.
fn show(running: &mut Running, view: PromptView) {
    running.feeds.polkit.send_replace(Polkit {
        registration: Registration::Registered,
        current: Some(view),
        waiting: 0,
    });
}

/// The agent has no request.
fn clear(running: &mut Running) {
    running.feeds.polkit.send_replace(Polkit::default());
}

/// The commands the prompt has sent so far.
fn sent(running: &mut Running) -> Vec<PolkitCommand> {
    std::iter::from_fn(|| running.feeds.polkit_commands.try_recv().ok()).collect()
}

/// A request asking for ada's password.
fn request(cookie: &str, phase: PromptPhase, last: LastTry) -> PromptView {
    PromptView {
        cookie: Cookie(cookie.to_owned()),
        action_id: ActionId("org.freedesktop.systemd1.manage-units".to_owned()),
        message: "Authentication is required to start 'sshd.service'.".to_owned(),
        icon: String::new(),
        details: vec![("unit".to_owned(), "sshd.service".to_owned())],
        user: UserName("ada".to_owned()),
        phase,
        last,
        notes: Vec::new(),
    }
}

/// The helper's password prompt.
fn asking() -> PromptPhase {
    PromptPhase::Asking {
        prompt: "Password: ".to_owned(),
        echo: Echo::Hidden,
    }
}

/// The prompt showing `view`, once its sheet has settled.
fn shown(view: PromptView) -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    show(&mut running, view);
    settle(&mut running.harness, |h| h.count(".ds-polkit") == 1);
    running.harness.advance(Duration::from_millis(600));
    running
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

#[test]
fn with_no_request_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".ds-polkit"), 0);
}

#[test]
fn a_request_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    show(&mut running, request("c1", asking(), LastTry::None));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".ds-polkit"), 0);
}

#[test]
fn a_request_draws_quires_prompt_on_a_narrow_sheet() {
    let running = shown(request("c1", asking(), LastTry::None));
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.attr(".ds-sheet", "data-width").as_deref(), Some("narrow"));
    assert_eq!(
        h.count(".ds-sheet#polkit-panel"),
        1,
        "the sheet carries the blur region's id"
    );
    assert_eq!(
        h.attr(".ds-sheet", "data-attach").as_deref(),
        Some("centre")
    );
    assert_eq!(h.count(".ds-scrim"), 0, "a sheet dims nothing");
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
    let sheet = h.rect(".ds-sheet").expect("the sheet is laid out"); // test-only
    let middle = sheet.origin.x.0 + sheet.size.width.0 / 2.0;
    assert!((middle - WIDTH / 2.0).abs() < 1.0, "centred: {sheet:?}");
    assert!(sheet.size.width.0 <= 340.5, "narrow: {sheet:?}");
}

#[test]
fn enter_answers_with_what_was_typed_and_escape_dismisses() {
    let mut running = shown(request("c1", asking(), LastTry::None));
    for c in "sill-fake".chars() {
        running.harness.send(Input::key(ShortcutKey::Char(c)));
    }
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(40));
    let asked = sent(&mut running);
    let [PolkitCommand::Answer { cookie, secret }] = asked.as_slice() else {
        panic!("one answer: {asked:?}");
    };
    assert_eq!(cookie, &Cookie("c1".to_owned()));
    assert_eq!(secret.expose(), "sill-fake");
    assert_eq!(format!("{secret:?}"), "Secret(..)", "never printed");
    assert!(
        !running.harness.html().contains("sill-fake"),
        "the secret is not in the markup"
    );
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
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
    show(&mut running, request("c1", asking(), LastTry::Wrong));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.harness.attr(".ds-polkit", "data-state").as_deref(),
        Some("wrong")
    );
    assert_eq!(
        running.harness.count(".ds-polkit .ds-password-field"),
        1,
        "the field stays"
    );
}

#[test]
fn a_request_that_goes_plays_the_sheet_out_then_draws_nothing() {
    let mut running = shown(request("c1", asking(), LastTry::None));
    clear(&mut running);
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
    show(&mut running, request("c2", asking(), LastTry::None));
    settle(&mut running.harness, |h| h.count(".ds-polkit") == 1);
    assert_eq!(
        running.harness.count(".ds-polkit"),
        1,
        "the next one enters"
    );
}
