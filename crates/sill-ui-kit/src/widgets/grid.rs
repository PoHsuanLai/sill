//! The vocabulary of where a desktop widget sits: a cell of the grid, a widget's place in it, and
//! the output a set of places belongs to. Data only; the grid's arithmetic is the desktop
//! layer's.

use sill_settings::WidgetKind;

/// A cell: column 0 is the leftmost, row 0 the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridSlot {
    pub column: u16,
    pub row: u16,
}

/// One widget's place: its top-left cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placement {
    pub kind: WidgetKind,
    pub slot: GridSlot,
}

/// Which output an entry belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputKey {
    /// An output with no name.
    Unnamed,
    /// The output's connector name.
    Named(String),
}

impl OutputKey {
    /// The key for an output named `name`; no name, or an empty one, is `Unnamed`.
    pub fn of(name: Option<&str>) -> OutputKey {
        match name {
            Some(name) if !name.is_empty() => OutputKey::Named(name.to_owned()),
            Some(_) | None => OutputKey::Unnamed,
        }
    }
}
