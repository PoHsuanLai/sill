//! The widget keys as quire's `WidgetLayout` and back (design/28 section 4.1). Until `layout.toml` (P2), sill keeps what it kept: the notification
//! center's widgets in order (`widgets.center`), the desktop's (`widgets.desktop_widgets`) and
//! each output's desktop cells in the state file (`desktop-widgets.json`). Each widget has one
//! size in each host ([`super::size_of`]): nothing is picked, so an add is made
//! at that size whatever the gallery asked, and a resize is refused. [`layout_of`] reads the
//! keys as one layout for Edit Widgets; [`edited`] applies one of its edits through quire's
//! `ds::widget::apply` and writes the result back as the same keys and places. Pure.
//!
//! sill places each kind at most once per surface (the drag, the regions and the state file
//! are keyed by kind), so adding a kind a surface already shows is refused
//! ([`Refused::Placed`]); two of one kind are the placements' instances in P2.

use ds::widget::{DesktopGrid, GridCell, Order, WidgetAt, WidgetEdit, WidgetLayout, apply};

use super::{WidgetHost, WidgetKind, kind_id, kind_of, size_of};
use crate::widgets::grid::{GridSlot, Placement};

/// What the layout is read from: the keys, and where this output's desktop widgets are now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetKeys {
    pub center: Vec<WidgetKind>,
    pub desktop: Vec<WidgetKind>,
    /// Each desktop widget's cell on this output (`desktop_widgets::place`); a widget that
    /// fits nowhere has none and is left out of the layout.
    pub placed: Vec<Placement>,
}

/// Why an edit was not made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// quire's layout refused it: no room, a taken cell, an unknown placement.
    #[error("{0}")]
    Layout(ds::widget::LayoutError),
    /// That surface already shows this kind.
    #[error("{0:?} is already placed there")]
    Placed(WidgetKind),
    /// Not a widget sill can place.
    #[error("{} is not a widget sill places", .0.as_str())]
    Foreign(ds::WidgetKind),
    /// A widget has one size; there is no other to change to.
    #[error("a widget has one size")]
    OneSize,
}

impl Refused {
    /// What Edit Widgets says under the gallery.
    pub fn words(&self) -> &'static str {
        match self {
            Refused::Layout(ds::widget::LayoutError::Full) => {
                "There is no room on the desktop for it."
            }
            Refused::Layout(ds::widget::LayoutError::Taken) => {
                "That place on the desktop is taken."
            }
            Refused::Layout(ds::widget::LayoutError::Unknown(_)) => {
                "That widget is no longer placed."
            }
            Refused::Placed(_) => "It is already there.",
            Refused::Foreign(_) => "This widget cannot be placed yet.",
            Refused::OneSize => "Each widget comes in one size.",
        }
    }
}

/// The keys as one layout: the center's widgets in order, then the desktop's at their cells.
pub fn layout_of(keys: &WidgetKeys) -> WidgetLayout {
    let center = keys.center.iter().enumerate().map(|(at, &kind)| {
        let order = Order(u16::try_from(at).unwrap_or(u16::MAX));
        (kind, WidgetHost::Tile, WidgetAt::Center(order))
    });
    let desktop = keys.desktop.iter().filter_map(|&kind| {
        let slot = keys.placed.iter().find(|p| p.kind == kind)?.slot;
        let cell = GridCell {
            column: slot.column,
            row: slot.row,
        };
        Some((kind, WidgetHost::Desktop, WidgetAt::Desktop(cell)))
    });
    center
        .chain(desktop)
        .fold(WidgetLayout::default(), |layout, (kind, host, at)| {
            layout.added(kind_id(kind), size_of(kind, host), at).0
        })
}

/// The keys after `edit` on a desktop of `grid`. An add is made at the kind's one size in its
/// host, whatever size the gallery sent; a resize is refused.
pub fn edited(
    keys: &WidgetKeys,
    edit: WidgetEdit,
    grid: DesktopGrid,
) -> Result<WidgetKeys, Refused> {
    let before = layout_of(keys);
    let edit = one_size(edit)?;
    if let WidgetEdit::Add { kind, host, .. } = &edit {
        let ours = kind_of(kind).ok_or_else(|| Refused::Foreign(kind.clone()))?;
        let shown = on(&before, *host).into_iter().any(|(k, _)| k == ours);
        if shown {
            return Err(Refused::Placed(ours));
        }
    }
    let after = apply(before.clone(), edit, grid).map_err(Refused::Layout)?;
    Ok(keys_of(keys, &before, &after))
}

/// `edit` with an add's size the kind's one size in its host; a resize is [`Refused::OneSize`].
fn one_size(edit: WidgetEdit) -> Result<WidgetEdit, Refused> {
    match edit {
        WidgetEdit::Add { kind, host, size } => {
            let size = kind_of(&kind).map_or(size, |ours| size_of(ours, host));
            Ok(WidgetEdit::Add { kind, host, size })
        }
        WidgetEdit::Resize(..) => Err(Refused::OneSize),
        WidgetEdit::Remove(id) => Ok(WidgetEdit::Remove(id)),
        WidgetEdit::Move(id, at) => Ok(WidgetEdit::Move(id, at)),
    }
}

/// The kinds on `host` in `layout`, in the surface's order, with their positions.
fn on(layout: &WidgetLayout, host: WidgetHost) -> Vec<(WidgetKind, WidgetAt)> {
    let mut items: Vec<(WidgetKind, WidgetAt)> = layout
        .items()
        .iter()
        .filter(|item| item.at.host() == host)
        .filter_map(|item| Some((kind_of(&item.kind)?, item.at)))
        .collect();
    // The column by its order; the desktop keeps the layout's order (the key's, new ones last).
    items.sort_by_key(|(_, at)| match *at {
        WidgetAt::Center(order) => order.0,
        WidgetAt::Desktop(_) => 0,
    });
    items
}

/// `keys` with `before`'s widgets as `after` has them.
fn keys_of(keys: &WidgetKeys, before: &WidgetLayout, after: &WidgetLayout) -> WidgetKeys {
    let now_desktop = on(after, WidgetHost::Desktop);
    let was_desktop = on(before, WidgetHost::Desktop);
    let held = |kind: WidgetKind| now_desktop.iter().any(|(k, _)| *k == kind);
    let unplaced = |kind: WidgetKind| was_desktop.iter().all(|(k, _)| *k != kind);
    let desktop: Vec<WidgetKind> = keys
        .desktop
        .iter()
        .copied()
        .filter(|&kind| held(kind) || unplaced(kind))
        .chain(
            now_desktop
                .iter()
                .map(|(kind, _)| *kind)
                .filter(|kind| !keys.desktop.contains(kind)),
        )
        .collect();
    let placed = now_desktop
        .iter()
        .filter_map(|&(kind, at)| match at {
            WidgetAt::Desktop(cell) => Some(Placement {
                kind,
                slot: GridSlot {
                    column: cell.column,
                    row: cell.row,
                },
            }),
            WidgetAt::Center(_) => None,
        })
        .collect();
    WidgetKeys {
        center: on(after, WidgetHost::Tile)
            .into_iter()
            .map(|(kind, _)| kind)
            .collect(),
        desktop,
        placed,
    }
}

#[cfg(test)]
mod tests;
