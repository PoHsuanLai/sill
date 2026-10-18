//! What the display trial draws and sends, on the harness over a `DisplayTrialLink` the test
//! writes as the daemon does: nothing until the compositor opens a trial, or while the banners are
//! on another output; then quire's `Alert` "Keep these display settings?" with a line that counts
//! the seconds down and two buttons, Revert (the default, and Escape) and Keep Changes, each
//! writing its answer to the link once; a restart of the trial starts the count over; the
//! compositor's end of the trial takes the sheet down; and nothing answers before the arming
//! time. All on the harness's virtual clock; nothing here reaches a compositor.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use shell_host::OutputId;
use sill_model::desktop::{DisplayTrial, TrialAnswer, TrialId};
use sill_overlays::DisplayTrialSheet;
use sill_overlays::arming::QUESTION_ARM_AFTER;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::{BannerLink, BannerOutput, DisplayTrialLink};

type Running = SurfaceHarness<DisplayTrialSheet>;

const REVERT: &str = "#display-trial-revert";
const KEEP: &str = "#display-trial-keep";

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn trial(id: u32, seconds: u32) -> DisplayTrial {
    DisplayTrial {
        trial: TrialId::new(id),
        seconds,
    }
}

fn start(on: OutputId) -> Running {
    let banners = BannerLink::new();
    banners.set(BannerOutput::On(on));
    SurfaceHarness::<DisplayTrialSheet>::setup(DisplayTrialLink::new(banners), ())
        .viewport(VIEWPORT)
        .start()
}

fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_harness::harness::settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The sheet drawn for trial 4 of 15 seconds, with no time passed beyond what drawing took.
fn drawn() -> Running {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(40));
    running.link.begin(trial(4, 15));
    settle(&mut running.harness, |h| h.count(".ds-alert") == 1);
    running
}

/// The sheet once it takes answers.
fn up() -> Running {
    let mut running = drawn();
    running
        .harness
        .advance(QUESTION_ARM_AFTER + Duration::from_millis(100));
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

fn says(running: &Running, line: &str) -> bool {
    running.harness.html().contains(line)
}

#[test]
fn until_the_compositor_opens_a_trial_it_draws_nothing() {
    let mut running = start(fake::output().id);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn a_trial_for_another_output_draws_nothing_here() {
    let mut running = start(OutputId(99));
    running.harness.advance(Duration::from_millis(40));
    running.link.begin(trial(4, 15));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".ds-alert"), 0);
}

#[test]
fn the_trial_asks_whether_to_keep_the_displays_with_the_time_left_and_two_buttons() {
    let running = drawn();
    let html = running.harness.html();
    for word in [
        "Keep these display settings?",
        "Reverting in 15 seconds",
        "Revert",
        "Keep Changes",
    ] {
        assert!(html.contains(word), "{word}: {html}");
    }
    assert_eq!(running.harness.count(REVERT), 1, "{html}");
    assert_eq!(running.harness.count(KEEP), 1, "{html}");
}

#[test]
fn the_line_counts_down_each_second_and_says_one_second_in_the_singular() {
    let mut running = drawn();
    running.harness.advance(Duration::from_millis(2_000));
    assert!(
        says(&running, "Reverting in 13 seconds"),
        "{}",
        running.harness.html()
    );
    running.harness.advance(Duration::from_millis(11_000));
    assert!(
        says(&running, "Reverting in 2 seconds"),
        "{}",
        running.harness.html()
    );
    running.harness.advance(Duration::from_millis(1_000));
    assert!(
        says(&running, "Reverting in 1 second"),
        "{}",
        running.harness.html()
    );
    assert!(!says(&running, "1 seconds"), "{}", running.harness.html());
}

#[test]
fn each_button_writes_its_answer_once_and_takes_the_sheet_down() {
    for (button, answer) in [(KEEP, TrialAnswer::Keep), (REVERT, TrialAnswer::Revert)] {
        let mut running = up();
        assert_eq!(
            running.link.last_answer(),
            None,
            "{button}: nothing before the press"
        );
        click(&mut running, button);
        let last = running.link.last_answer().expect("answered");
        assert_eq!(
            (last.seq, last.trial, last.answer),
            (1, TrialId::new(4), answer),
            "{button}"
        );
        settle(&mut running.harness, |h| h.count(".ds-alert") == 0);
    }
}

#[test]
fn escape_and_return_both_revert() {
    for (name, key) in [
        ("Escape", ShortcutKey::Escape),
        ("Return", ShortcutKey::Enter),
    ] {
        let mut running = up();
        running.harness.send(Input::key(key));
        running.harness.advance(Duration::from_millis(40));
        let last = running.link.last_answer().expect("answered");
        assert_eq!(
            (last.trial, last.answer),
            (TrialId::new(4), TrialAnswer::Revert),
            "{name}: Revert is the default and the Escape button"
        );
    }
}

#[test]
fn the_compositors_end_of_the_trial_takes_the_sheet_down_unanswered() {
    let mut running = up();
    running.link.end(TrialId::new(4));
    settle(&mut running.harness, |h| h.count(".ds-alert") == 0);
    assert_eq!(running.link.last_answer(), None);
    assert_eq!(running.harness.count(".ds-sheet"), 0);
}

#[test]
fn the_end_of_another_trial_leaves_the_sheet_up() {
    let mut running = up();
    running.link.end(TrialId::new(9));
    running.harness.advance(Duration::from_millis(400));
    assert_eq!(running.harness.count(".ds-alert"), 1);
}

#[test]
fn a_repeated_trial_with_the_same_number_starts_the_count_over() {
    let mut running = up();
    running.harness.advance(Duration::from_millis(4_500));
    assert!(
        says(&running, "Reverting in 10 seconds"),
        "{}",
        running.harness.html()
    );
    running.link.begin(trial(4, 15));
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".ds-alert"), 1, "still one question");
    assert!(
        says(&running, "Reverting in 15 seconds"),
        "{}",
        running.harness.html()
    );
    running
        .harness
        .advance(QUESTION_ARM_AFTER + Duration::from_millis(100));
    click(&mut running, KEEP);
    let last = running.link.last_answer().expect("answered");
    assert_eq!(
        (last.trial, last.answer),
        (TrialId::new(4), TrialAnswer::Keep)
    );
}

#[test]
fn a_new_trial_takes_the_place_of_the_one_before_and_answers_for_itself() {
    let mut running = up();
    running.link.begin(trial(5, 10));
    running
        .harness
        .advance(QUESTION_ARM_AFTER + Duration::from_millis(200));
    assert!(
        says(&running, "Reverting in 10 seconds"),
        "{}",
        running.harness.html()
    );
    click(&mut running, REVERT);
    let last = running.link.last_answer().expect("answered");
    assert_eq!(last.trial, TrialId::new(5));
}

#[test]
fn nothing_answers_until_the_sheet_has_been_up_for_the_arming_time() {
    // Three inputs and their settling take about 120 ms, well inside the arming time.
    const EARLY: Duration = Duration::from_millis(200);
    let mut running = drawn();
    running.harness.advance(EARLY);
    click(&mut running, KEEP);
    click(&mut running, REVERT);
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        running.link.last_answer(),
        None,
        "a click, Return or Escape before the arming time answers nothing: {}",
        running.harness.html()
    );
    assert_eq!(running.harness.count(".ds-alert"), 1, "still asked");
    running.harness.advance(QUESTION_ARM_AFTER);
    click(&mut running, KEEP);
    let last = running.link.last_answer().expect("answered");
    assert_eq!(
        last.answer,
        TrialAnswer::Keep,
        "the same click after the arming time answers"
    );
}
