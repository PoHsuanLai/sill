//! What the voice consent sheet draws and sends, on the harness over the fake voice service:
//! nothing until the service asks, or while the sheets are on another output; with the question,
//! quire's `Sheet` with the one sentence of what listening means and two buttons; Allow, Not Now
//! and Escape each sending their answer once; nothing after the service stops asking. All on the
//! harness's virtual clock; nothing here reaches a bus.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use shell_host::OutputId;
use sill_model::voice::{ConsentAnswer, ConsentSheet, Voice, VoiceCommand};
use sill_overlays::VoiceConsentSheet;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{SheetLink, SheetOutput};

type Running = SurfaceHarness<VoiceConsentSheet>;

const ALLOW: &str = "#voice-consent-allow";
const NOT_NOW: &str = "#voice-consent-not-now";

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn start(on: OutputId) -> Running {
    let link = SheetLink::new();
    link.set(SheetOutput::On(on));
    SurfaceHarness::<VoiceConsentSheet>::setup(link, ())
        .viewport(VIEWPORT)
        .start()
}

fn asking(running: &mut Running, sheet: ConsentSheet) {
    running.feeds.voice.send_replace(Voice {
        sheet,
        ..Voice::default()
    });
}

fn sent(running: &mut Running) -> Vec<VoiceCommand> {
    std::iter::from_fn(|| running.feeds.voice_commands.try_recv().ok()).collect()
}

fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

fn up() -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    asking(&mut running, ConsentSheet::Asking);
    settle(&mut running.harness, |h| h.count(".sill-consent") == 1);
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
fn until_the_service_asks_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
}

#[test]
fn a_question_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    asking(&mut running, ConsentSheet::Asking);
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".sill-consent"), 0);
}

#[test]
fn the_sheet_says_what_listening_means_and_offers_two_answers() {
    let running = up();
    let html = running.harness.html();
    assert_eq!(running.harness.count(".ds-sheet"), 1, "{html}");
    assert!(
        html.contains("The assistant listens only while you hold"),
        "{html}"
    );
    assert!(html.contains("is not kept"), "{html}");
    assert_eq!(running.harness.count(ALLOW), 1, "{html}");
    assert_eq!(running.harness.count(NOT_NOW), 1, "{html}");
}

#[test]
fn allow_and_not_now_each_send_their_answer() {
    for (button, answer) in [
        (ALLOW, ConsentAnswer::Allow),
        (NOT_NOW, ConsentAnswer::NotNow),
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
            vec![VoiceCommand::Answer(answer)],
            "{button}"
        );
    }
}

#[test]
fn escape_is_not_now() {
    let mut running = up();
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
        vec![VoiceCommand::Answer(ConsentAnswer::NotNow)]
    );
}

#[test]
fn the_sheet_leaves_once_the_service_stops_asking() {
    let mut running = up();
    asking(&mut running, ConsentSheet::Closed);
    settle(&mut running.harness, |h| h.count(".ds-sheet") == 0);
    assert_eq!(running.harness.count(".sill-consent"), 0);
}
