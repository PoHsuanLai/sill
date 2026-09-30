//! A detail pane's list (networks, devices) as a quire `List` of settings-height `Row`s: each row's value, glyph,
//! words, trailing mark, operation phase and disc as data, so the pages build them purely and
//! test them. Rows are keyed by their item (an SSID, an address), so a row's detail state stays
//! with its item when the list reorders.

use crate::control_center::item_phase::{ItemPhase, RowDisc};
use dioxus::prelude::*;
use ds::{
    Accessory, Availability, BatteryState, Fraction, Icon, List, ListItem, ListStyle, Row,
    RowLeading, RowSize, RowState, Selection, TextLine,
};
use sill_model::units::Percent;

/// What a row ends in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RowMark {
    Nothing,
    /// A secured network: a lock (a joining row's spinner takes its place).
    Lock,
    /// A connected device's battery (design/26 G21).
    Battery(Percent),
}

/// One row: its key, what choosing it yields, its glyph, title and detail (empty for none), its
/// mark, the operation on its item and its disc.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowSpec<T> {
    pub key: String,
    pub value: T,
    pub glyph: Icon,
    pub title: String,
    pub detail: String,
    pub mark: RowMark,
    pub phase: ItemPhase,
    pub disc: RowDisc,
    pub availability: Availability,
}

/// The rows, in order, named `label`; choosing one calls `onpick` with its value.
#[component]
pub(crate) fn RowList<T: Clone + PartialEq + 'static>(
    label: String,
    rows: Vec<RowSpec<T>>,
    onpick: EventHandler<T>,
) -> Element {
    let items: Vec<ListItem<String>> = rows
        .into_iter()
        .map(|row| {
            let detail = (!row.detail.is_empty()).then(|| TextLine::from(row.detail.clone()));
            let value = row.value.clone();
            let content = rsx! {
                Row {
                    leading: leading(row.glyph, row.disc),
                    title: row.title.clone(),
                    detail,
                    accessory: accessory(row.mark),
                    state: RowState { availability: availability(row.availability, row.phase), ..RowState::default() },
                    size: RowSize::Settings,
                    onclick: move |_| onpick.call(value.clone()),
                }
            };
            ListItem::row(row.key.clone(), row.title.clone(), content)
        })
        .collect();
    rsx! {
        List::<String> { label, items, style: ListStyle::Inset }
    }
}

/// quire's leading element for a row's glyph and disc.
fn leading(glyph: Icon, disc: RowDisc) -> RowLeading {
    match disc {
        RowDisc::None => RowLeading::Icon(glyph),
        RowDisc::Off => RowLeading::Disc(glyph, Selection::Unselected),
        RowDisc::On => RowLeading::Disc(glyph, Selection::Selected),
    }
}

/// A row is busy while its operation runs.
fn availability(own: Availability, phase: ItemPhase) -> Availability {
    match phase {
        ItemPhase::Pending(_) => Availability::Busy,
        ItemPhase::Rest | ItemPhase::Succeeded(_) | ItemPhase::Failed(_) => own,
    }
}

/// quire's accessory for `mark`.
fn accessory(mark: RowMark) -> Accessory {
    match mark {
        RowMark::Nothing => Accessory::None,
        RowMark::Lock => Accessory::Glyph(Icon::Lock),
        RowMark::Battery(level) => Accessory::Battery(BatteryState {
            level: Fraction(level.0.min(100) * 10),
            ..BatteryState::default()
        }),
    }
}
