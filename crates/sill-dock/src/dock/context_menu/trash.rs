//! The Trash tile's rows (design/10 §10.3.9): Open, and Empty Trash, which is off while the
//! Trash holds nothing.

use ds::prelude::Availability;

use super::DockMenuRow;

/// The Trash tile's rows; `empty_trash` says whether Empty Trash can be chosen.
pub fn rows(empty_trash: Availability) -> Vec<DockMenuRow> {
    vec![DockMenuRow::OpenTrash, DockMenuRow::EmptyTrash(empty_trash)]
}

#[cfg(test)]
#[path = "trash_tests.rs"]
mod tests;
