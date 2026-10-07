//! A registry, one trait, a picker, placements as data (design/23-WIDGETS.md section 9.1): the
//! pattern the widgets use, kept generic here so the other placeable surfaces (Spotlight's
//! categories, the control center's modules, the menu bar's items) can reuse its data half. A
//! surface names what can be placed (its registry of kinds), draws each kind through one trait,
//! offers a picker over the registry, and keeps what the person placed as data, a list of
//! [`Placed`]s the host stores in its settings, never in code.
//!
//! Only the data half is generic: a placement is a kind, a size and a position, and the list's
//! edits (add, remove, resize, move) are pure. Where a position is legal (a free grid cell, an
//! order in a column) is each surface's own rule, applied before an edit reaches the list.

pub(crate) mod placement;

pub use placement::PlacementId;
