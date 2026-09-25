//! What the power menu draws: nothing while hidden; open, quire's Sheet over its full-output
//! scrim, the question for the focused button, the four buttons in design/20 §1.8's order and
//! variants, the key hints, and the blur element covering exactly the sheet.

mod power_menu_support;

use std::time::Duration;

use power_menu_support::{HEIGHT, VIEWPORT, WIDTH, look, nth_button, opened, start};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::power_menu::{PANEL_ID, Preselect, everything_offered};

#[test]
fn hidden_it_draws_nothing() {
    let mut running = start(ShellFile::default(), everything_offered());
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(look(&running.harness).as_deref(), Some("closed"));
    assert_eq!(running.harness.count(".ds-sheet"), 0);
    assert_eq!(running.harness.count(".ds-scrim"), 0);
    assert_eq!(running.harness.count(".ds-button"), 0);
}

#[test]
fn open_it_draws_the_sheet_the_buttons_and_the_hints() {
    let running = opened(
        ShellFile::default(),
        everything_offered(),
        Preselect::Default,
    );
    let h = &running.harness;
    assert_eq!(h.count(".ds-sheet"), 1);
    assert_eq!(h.count(".ds-scrim"), 1);
    assert_eq!(
        h.text_of(".sill-power-menu-prompt").as_deref(),
        Some("Shut down this computer now?")
    );
    let buttons = [
        ("Log Out", "secondary"),
        ("Restart", "danger"),
        ("Shut Down", "primary"),
        ("Suspend", "secondary"),
    ];
    assert_eq!(
        h.count(".sill-power-menu-buttons .ds-button"),
        buttons.len()
    );
    for (n, (label, variant)) in buttons.iter().enumerate() {
        let button = nth_button(n + 1);
        assert_eq!(h.text_of(&button).as_deref(), Some(*label));
        assert_eq!(
            h.attr(&button, "data-variant").as_deref(),
            Some(*variant),
            "{label}"
        );
    }
    assert_eq!(h.count(".sill-power-menu-hint"), 3);
    assert_eq!(h.count(".sill-power-menu-hint .ds-kbd"), 4, "← →, ↵, Esc");
}

#[test]
fn the_scrim_covers_the_output_and_the_blur_element_is_the_sheet() {
    let running = opened(
        ShellFile::default(),
        everything_offered(),
        Preselect::Default,
    );
    let h = &running.harness;
    let scrim = h.rect(".ds-scrim").expect("the scrim is laid out"); // test-only
    assert_eq!(
        (scrim.size.width.0, scrim.size.height.0),
        (WIDTH, HEIGHT),
        "the whole output dims"
    );
    let sheet = h.rect(".ds-sheet").expect("the sheet is laid out"); // test-only
    let panel = h
        .rect(&format!("#{PANEL_ID}"))
        .expect("the blur element is laid out"); // test-only
    assert_eq!(panel, sheet, "the blur region is the sheet's box");
    let middle = sheet.origin.x.0 + sheet.size.width.0 / 2.0;
    assert!(
        (middle - WIDTH / 2.0).abs() < 1.0,
        "centred across the output: {sheet:?}"
    );
    assert!(sheet.size.height.0 > 100.0, "{sheet:?}");
}

/// A picture of the open menu for eyes, under the test target's scratch directory.
#[test]
fn the_open_menu_renders() {
    let mut running = opened(
        ShellFile::default(),
        everything_offered(),
        Preselect::Default,
    );
    let image = running
        .harness
        .render()
        .expect("the harness renders the menu"); // test-only
    assert_eq!(
        (image.width(), image.height()),
        (VIEWPORT.width, VIEWPORT.height)
    );
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("power-menu-harness.png");
    image.save(&path).expect("the picture saves"); // test-only
}
