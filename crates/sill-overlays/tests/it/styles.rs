//! The person's styles on a surface: its document names itself (`data-surface`), draws the
//! feed's sheets each in a `<style data-ds-style>` of its own in the order look, desktop, app,
//! redraws them when the feed changes, drops an element when its file goes, and draws none while
//! "Use my styles" is off.

use std::time::Duration;

use ds::style::kit::{CustomStyles, UserStyle};
use ds_settings::Environment;
use sill_overlays::Osd;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::OsdLink;
use sill_ui_kit::surface::Surface;

const RED: &str = ".ds { --accent: red }";
const BLUE: &str = ".ds { --accent: blue }";
const GREEN: &str = ".ds { --accent: green }";

fn settle(running: &mut SurfaceHarness<Osd>) {
    running.harness.advance(Duration::from_millis(60));
}

fn drawn(running: &SurfaceHarness<Osd>, slot: &str) -> usize {
    running
        .harness
        .html()
        .matches(&format!("data-ds-style=\"{slot}\""))
        .count()
}

fn all_drawn(running: &SurfaceHarness<Osd>) -> usize {
    running.harness.html().matches("data-ds-style").count()
}

fn with_custom_styles(custom: CustomStyles) -> Environment {
    let mut environment = Environment::default();
    environment.settings.appearance.custom_styles = custom;
    environment
}

#[test]
fn a_surface_draws_both_sheets_in_order_and_follows_the_app_one() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ())
        .desktop_style(RED)
        .app_style(BLUE)
        .start();
    settle(&mut running);
    assert_eq!(
        running.harness.attr(".ds", "data-surface").as_deref(),
        Some(Osd::NAME)
    );
    assert_eq!((drawn(&running, "desktop"), drawn(&running, "app")), (1, 1));
    assert_eq!(drawn(&running, "look"), 0, "the look is empty for now");
    let html = running.harness.html();
    let (desktop, app) = (html.find(RED), html.find(BLUE));
    assert!(
        desktop < app && desktop.is_some(),
        "desktop before app: {html}"
    );

    running
        .styles
        .send_modify(|styles| styles.app = UserStyle(GREEN.to_owned()));
    settle(&mut running);
    let html = running.harness.html();
    assert!(
        html.contains(GREEN) && !html.contains(BLUE) && html.contains(RED),
        "{html}"
    );
    assert_eq!(
        all_drawn(&running),
        2,
        "one element each, its text replaced"
    );

    running
        .styles
        .send_modify(|styles| styles.app = UserStyle::default());
    settle(&mut running);
    assert_eq!(drawn(&running, "app"), 0, "a deleted file draws nothing");
    assert_eq!(drawn(&running, "desktop"), 1);
}

#[test]
fn a_change_to_the_desktop_wide_file_reaches_the_surface() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ()).start();
    settle(&mut running);
    assert_eq!(all_drawn(&running), 0);
    running
        .styles
        .send_modify(|styles| styles.desktop = UserStyle(RED.to_owned()));
    settle(&mut running);
    assert_eq!(drawn(&running, "desktop"), 1);
    assert!(running.harness.html().contains(RED));
}

#[test]
fn use_my_styles_off_removes_both_sheets_and_on_brings_them_back() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ())
        .desktop_style(RED)
        .app_style(BLUE)
        .start();
    settle(&mut running);
    assert_eq!(all_drawn(&running), 2);

    running.set_appearance(with_custom_styles(CustomStyles::Off));
    settle(&mut running);
    assert_eq!(all_drawn(&running), 0);
    let html = running.harness.html();
    assert!(!html.contains(RED) && !html.contains(BLUE), "{html}");

    running.set_appearance(with_custom_styles(CustomStyles::Use));
    settle(&mut running);
    assert_eq!(all_drawn(&running), 2);
}

#[test]
fn a_surface_without_style_files_draws_no_element() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ()).start();
    settle(&mut running);
    assert_eq!(all_drawn(&running), 0);
}
