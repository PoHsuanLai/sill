//! The Force Quit Applications panel on the harness, following a `ForceQuitLink` the test writes
//! as the daemon does. Hidden it draws nothing; open it lists the apps with "(not responding)"
//! after a hung one's name and starts with the hung app selected, the Force Quit button asks
//! "Do you want to force quit ...?" first, the question's Force Quit writes the app on the
//! link and its Cancel does not, and Escape answers the question before it closes the panel.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Harness, Input, Viewport};
use sill_model::desktop::{ForceQuitRow, Standing};
use sill_overlays::ForceQuit;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::links::ForceQuitLink;

type Running = SurfaceHarness<ForceQuit>;

/// A small output.
const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

fn row(app: &str, standing: Standing) -> ForceQuitRow {
    ForceQuitRow {
        app: app.to_owned(),
        standing,
    }
}

/// The panel with nothing listed, closed.
fn start() -> Running {
    let mut running = SurfaceHarness::<ForceQuit>::setup(ForceQuitLink::new(), ())
        .viewport(VIEWPORT)
        .start();
    running.harness.advance(Duration::from_millis(40));
    running
}

/// The panel open over `rows`, once its sheet has settled.
fn opened(rows: Vec<ForceQuitRow>) -> Running {
    let mut running = start();
    running.link.set_rows(rows);
    running.link.open(fake::output().id);
    running.harness.advance(Duration::from_millis(600));
    running
}

fn hung_and_well() -> Vec<ForceQuitRow> {
    vec![
        row("mail", Standing::Responding),
        row("term", Standing::NotResponding),
    ]
}

fn press(harness: &mut Harness, selector: &str) {
    let at = harness
        .centre(selector)
        .unwrap_or_else(|| panic!("{selector} is not laid out:\n{}", harness.html()));
    harness.send(Input::click(at));
    harness.advance(Duration::from_millis(600));
}

fn asking(harness: &Harness) -> bool {
    harness.count(".ds-alert") == 1
}

#[test]
fn hidden_it_draws_nothing() {
    let running = start();
    assert_eq!(running.harness.count(".sill-fq"), 0);
    assert_eq!(running.harness.count(".ds-sheet"), 0);
}

#[test]
fn open_it_lists_the_apps_and_says_which_do_not_answer() {
    let running = opened(hung_and_well());
    let html = running.harness.html();
    assert_eq!(running.harness.count(".sill-fq"), 1);
    assert_eq!(running.harness.count(".ds-row"), 2);
    assert!(html.contains("Force Quit Applications"), "{html}");
    assert!(html.contains("term (not responding)"), "{html}");
    assert!(!html.contains("mail (not responding)"), "{html}");
}

#[test]
fn force_quit_asks_first_and_the_hung_app_is_the_one_asked_about() {
    let mut running = opened(hung_and_well());
    assert!(!asking(&running.harness), "no question before the press");
    press(&mut running.harness, "#force-quit-button");
    assert!(asking(&running.harness));
    let html = running.harness.html();
    assert!(html.contains("Do you want to force quit"), "{html}");
    assert!(html.contains("term"), "{html}");
    assert!(
        html.contains("You will lose any unsaved changes."),
        "{html}"
    );
    assert_eq!(running.link.last_asked(), None, "asking ends nothing");
}

#[test]
fn confirming_writes_the_app_and_cancelling_does_not() {
    let mut running = opened(hung_and_well());
    press(&mut running.harness, "#force-quit-button");
    press(&mut running.harness, "#force-quit-cancel");
    assert!(!asking(&running.harness), "Cancel answers the question");
    assert_eq!(running.link.last_asked(), None);

    press(&mut running.harness, "#force-quit-button");
    press(&mut running.harness, "#force-quit-confirm");
    let asked = running.link.last_asked().expect("confirmed");
    assert_eq!((asked.seq, asked.app.as_str()), (1, "term"));
    assert!(!asking(&running.harness));
    assert!(
        running.link.now().opening().is_some(),
        "the panel stays open for the next app"
    );
}

#[test]
fn nothing_hung_nothing_selected_so_force_quit_asks_nothing() {
    let mut running = opened(vec![row("mail", Standing::Responding)]);
    press(&mut running.harness, "#force-quit-button");
    assert!(!asking(&running.harness));
}

#[test]
fn escape_answers_the_question_first_and_then_closes_the_panel() {
    let mut running = opened(hung_and_well());
    press(&mut running.harness, "#force-quit-button");
    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(600));
    assert!(!asking(&running.harness), "the question is answered");
    assert!(running.link.now().opening().is_some(), "the panel stays");

    running.harness.send(Input::key(ShortcutKey::Escape));
    running.harness.advance(Duration::from_millis(600));
    assert!(running.link.now().opening().is_none(), "the panel closes");
    assert_eq!(running.harness.count(".sill-fq"), 0, "and leaves");
}
