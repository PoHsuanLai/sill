//! Table tests for the Trash tile's menu.

use ds::prelude::*;

use super::rows;
use crate::dock::context_menu::{DockMenuRow, menu_entries};
use crate::dock::pick::DockPick;
use sill_settings::DockMenuOrder;

fn entries(empty_trash: Availability) -> Vec<(String, DockPick, Availability)> {
    menu_entries(&rows(empty_trash), None, &[], DockMenuOrder::AppleOrder)
        .into_iter()
        .filter_map(|e| match e {
            MenuItem::Item {
                title,
                value,
                availability,
                ..
            } => Some((title, value, availability)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_trash_offers_open_and_empty_trash() {
    assert_eq!(
        rows(Availability::Enabled),
        [
            DockMenuRow::OpenTrash,
            DockMenuRow::EmptyTrash(Availability::Enabled)
        ]
    );
    assert_eq!(
        entries(Availability::Enabled),
        [
            ("Open".to_owned(), DockPick::OpenTrash, Availability::Enabled),
            (
                "Empty Trash".to_owned(),
                DockPick::EmptyTrash,
                Availability::Enabled
            ),
        ]
    );
}

#[test]
fn empty_trash_is_off_while_the_trash_is_empty() {
    let listed = entries(Availability::Disabled);
    assert_eq!(listed[0].2, Availability::Enabled);
    assert_eq!(listed[1].2, Availability::Disabled);
}
