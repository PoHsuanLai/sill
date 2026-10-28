//! The dock's own menu, on its separator or empty background (design/10 §10.3.7): Turn Hiding
//! On / Off, Turn Magnification On / Off, then Dock Settings. The Mac's Position on Screen and
//! Minimise Windows Using rows are not here: the dock has no setting for either (it stays at the
//! bottom, and has one minimise effect).

use sill_settings::OnOff;

use super::DockMenuRow;

/// The rows, given how hiding and magnification are set now.
pub fn rows(autohide: OnOff, magnification: OnOff) -> Vec<DockMenuRow> {
    vec![
        DockMenuRow::Hiding(autohide),
        DockMenuRow::Magnifying(magnification),
        DockMenuRow::Separator,
        DockMenuRow::DockSettings,
    ]
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
