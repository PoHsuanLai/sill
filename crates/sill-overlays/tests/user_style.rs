//! The person's `style.css` on a surface: its document names itself (`data-surface`), draws the
//! feed's stylesheet in `<style data-ds-user>`, redraws it when the feed changes and drops the
//! element when the file goes.

use ds_harness::{Driver, Query};
use std::time::Duration;

use ds_settings::UserStyle;
use sill_overlays::Osd;
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::OsdLink;
use sill_ui_kit::surface::Surface;

const RED: &str = ".ds { --accent: red }";
const BLUE: &str = ".ds { --accent: blue }";

fn settle(running: &mut SurfaceHarness<Osd>) {
    running.harness.advance(Duration::from_millis(60));
}

fn styles(running: &SurfaceHarness<Osd>) -> usize {
    running.harness.html().matches("data-ds-user").count()
}

#[test]
fn a_surface_draws_the_feeds_stylesheet_and_follows_it() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ())
        .user_style(RED)
        .start();
    settle(&mut running);
    assert_eq!(
        running.harness.attr(".ds", "data-surface").as_deref(),
        Some(Osd::NAME)
    );
    assert_eq!(styles(&running), 1);
    assert!(running.harness.html().contains(RED));

    running.user_style.send_replace(UserStyle(BLUE.to_owned()));
    settle(&mut running);
    let html = running.harness.html();
    assert!(html.contains(BLUE) && !html.contains(RED), "{html}");
    assert_eq!(styles(&running), 1, "one element, its text replaced");

    running.user_style.send_replace(UserStyle::default());
    settle(&mut running);
    assert_eq!(styles(&running), 0, "a deleted file draws nothing");
    assert!(!running.harness.html().contains(BLUE));
}

#[test]
fn a_surface_without_a_style_file_draws_no_element() {
    let mut running = SurfaceHarness::<Osd>::setup(OsdLink::new(), ()).start();
    settle(&mut running);
    assert_eq!(styles(&running), 0);
}
