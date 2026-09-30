//! The palette's card is as tall as its content (quire's macOS polish pass), and the panel
//! publishes it as the part of the panel that takes presses, which the catcher leaves out: the
//! card's measured bottom edge, in a `ds_harness::Harness`, through the harness's own measurer.

mod launcher_harness;

use ds::ShortcutKey;
use launcher_harness::{Press, open, settle};
use sill_launcher_ui::launcher::{CARD_ID, PANEL_HEIGHT};
use sill_ui_kit::links::PanelHit;

/// Advance until the panel has published its card (at most two seconds).
fn measured(open: &mut launcher_harness::Open) {
    let link = open.link.clone();
    for _ in 0..100 {
        if link.hit() != PanelHit::Whole {
            break;
        }
        open.harness.advance(std::time::Duration::from_millis(20));
    }
}

/// The card's height as the harness lays it out, whole pixels up.
fn card_height(open: &launcher_harness::Open) -> i32 {
    let rect = open
        .harness
        .rect(&format!("#{CARD_ID}"))
        .expect("the card is laid out");
    assert_eq!(rect.origin.y.0, 0.0, "the card starts at the panel's top");
    rect.size.height.0.ceil() as i32
}

#[test]
fn a_short_list_is_a_short_card_and_the_panel_takes_only_it() {
    let mut open = open("6*7");
    measured(&mut open);
    let height = card_height(&open);
    assert!(
        height < i32::try_from(PANEL_HEIGHT).unwrap_or(i32::MAX) / 2,
        "one row is a card of {height} px"
    );
    assert_eq!(
        open.harness
            .attr(&format!("#{CARD_ID}"), "data-corner")
            .as_deref(),
        Some("squircle")
    );
    assert_eq!(open.link.hit(), PanelHit::Card { height });
}

#[test]
fn more_rows_grow_the_card_and_the_actions_menu_takes_the_panel() {
    // One harness at a time: each installs its virtual clock on the thread while it lives.
    let one = {
        let mut short = open("6*7");
        measured(&mut short);
        card_height(&short)
    };
    let mut long = open("fi");
    measured(&mut long);
    let four = card_height(&long);
    assert!(four > one, "four rows {four} px, one {one} px");
    assert_eq!(long.link.hit(), PanelHit::Card { height: four });
    long.press(Press::Actions);
    settle(&mut long.harness, |h| h.count(".ds-popover") == 1);
    assert_eq!(
        long.link.hit(),
        PanelHit::Whole,
        "the menu may hang below the card"
    );
    long.harness.key(ShortcutKey::Escape);
    settle(&mut long.harness, |h| h.count(".ds-popover") == 0);
    measured(&mut long);
    assert_eq!(
        long.link.hit(),
        PanelHit::Card { height: four },
        "the menu closed"
    );
}
