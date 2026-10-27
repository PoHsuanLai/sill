//! Table tests for the dock's own menu.

use ds::prelude::*;
use sill_settings::{AutoHide, Magnification};

use super::rows;
use crate::dock::context_menu::{DockMenuRow, menu_entries};
use crate::dock::pick::DockPick;

fn listed(autohide: AutoHide, magnification: Magnification) -> Vec<(String, Option<DockPick>)> {
    menu_entries(&rows(autohide, magnification), None, &[])
        .into_iter()
        .map(|e| match e {
            MenuItem::Item { title, value, .. } => (title, Some(value)),
            _ => ("--".to_owned(), None),
        })
        .collect()
}

#[test]
fn the_rows_are_hiding_magnification_a_rule_and_dock_settings() {
    assert_eq!(
        rows(AutoHide::Off, Magnification::On),
        [
            DockMenuRow::Hiding(AutoHide::Off),
            DockMenuRow::Magnifying(Magnification::On),
            DockMenuRow::Separator,
            DockMenuRow::DockSettings,
        ]
    );
}

#[test]
fn the_toggles_read_by_the_state_they_will_set_and_dock_settings_opens_the_page() {
    let cases = [
        (
            AutoHide::Off,
            Magnification::On,
            "Turn Hiding On",
            "Turn Magnification Off",
        ),
        (
            AutoHide::On,
            Magnification::Off,
            "Turn Hiding Off",
            "Turn Magnification On",
        ),
    ];
    for (hiding, magnifying, want_hiding, want_magnifying) in cases {
        let menu = listed(hiding, magnifying);
        assert_eq!(
            menu,
            [
                (want_hiding.to_owned(), Some(DockPick::ToggleHiding)),
                (
                    want_magnifying.to_owned(),
                    Some(DockPick::ToggleMagnification)
                ),
                ("--".to_owned(), None),
                (
                    "Dock Settings…".to_owned(),
                    Some(DockPick::OpenDockSettings)
                ),
            ]
        );
    }
}
