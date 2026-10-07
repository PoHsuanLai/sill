//! The Batteries widget on the widget contract (design/23-WIDGETS.md sections 4.1 and 9.6): this
//! computer's battery and each device's, as the reference composes them. Small with one
//! device: the ring at the top left, the percentage as the card's hero figure. Small with
//! several: a 2 x 2 grid of rings, no numbers, an empty place a bare track. Medium: a row of up
//! to four rings, the percentage under each. Each ring holds the device's filled glyph
//! ([`crate::battery::device_glyph::DeviceGlyph`]).

use crate::battery::device_glyph::{Device, DeviceGlyph};
use crate::battery::ring::{BatteryRing, percent_text};
use crate::widget::contract::{NoIntent, Widget, WidgetContext, WidgetKind};
use crate::widget::kind::{WidgetHost, WidgetSize};
use dioxus::prelude::*;
use ds::base::vocab::Fraction;
use ds::base::word::Word;
use ds::components::content::label::{Label, LabelStyle};
use ds::components::content::status::battery_state::{BatteryPower, BatteryState, LowAt};
use ds::components::content::text_runs::TextLine;
use ds::style::icon::render::IconSize;
use serde::{Deserialize, Serialize};

/// The Batteries widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BatteryWidget;

/// One battery.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatteryCell {
    /// The device's name, what its ring is labelled for assistive technology.
    pub name: String,
    /// Its glyph.
    pub device: Device,
    /// Its charge, its power and where it reads low: the one state every battery drawing takes.
    pub state: BatteryState,
}

/// One moment of the Batteries widget.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryEntry {
    /// No reading yet: bare tracks, no numbers.
    #[default]
    Waiting,
    /// The batteries, this computer's first; the first four are drawn.
    Devices(Vec<BatteryCell>),
    /// Nothing reports a battery: a quiet line in the provider's words ("No batteries").
    Absent(String),
}

/// How many rings a grid or a row holds (the reference's widget shows up to four).
pub const MAX_RINGS: usize = 4;

/// How the widget lays out its rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(crate) enum BatteryLayout {
    /// Small, one battery: the ring and the hero figure.
    Solo,
    /// Small, several: a 2 x 2 grid of rings, no numbers.
    Grid,
    /// Medium: a row of rings, a figure under each.
    Row,
}

impl BatteryLayout {
    /// The layout for `size` with `count` batteries.
    pub(crate) fn of(size: WidgetSize, count: usize) -> Self {
        match (size, count) {
            (WidgetSize::Small, 1) => BatteryLayout::Solo,
            (WidgetSize::Small, _) => BatteryLayout::Grid,
            (WidgetSize::Medium | WidgetSize::Large, _) => BatteryLayout::Row,
        }
    }
}

impl Widget for BatteryWidget {
    type Entry = BatteryEntry;
    type Intent = NoIntent;

    fn kind() -> WidgetKind {
        WidgetKind::fixed("quire.battery")
    }

    fn name() -> TextLine {
        TextLine::from("Batteries")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    /// Small on the desktop (the ring and the hero figure, or the grid); Medium in the
    /// notification center, whose tiles span the column.
    fn size_in(host: WidgetHost) -> WidgetSize {
        match host {
            WidgetHost::Desktop => WidgetSize::Small,
            WidgetHost::Tile => WidgetSize::Medium,
        }
    }

    fn description() -> TextLine {
        TextLine::from("See the charge of this computer and your devices.")
    }

    fn placeholder(_size: WidgetSize) -> BatteryEntry {
        BatteryEntry::Waiting
    }

    fn preview(size: WidgetSize) -> BatteryEntry {
        let cell = |name: &str, device, level| BatteryCell {
            name: name.to_owned(),
            device,
            state: BatteryState {
                level: Fraction(level),
                power: BatteryPower::Battery,
                low_at: LowAt::default(),
            },
        };
        let laptop = cell("Laptop", Device::Laptop, 820);
        match size {
            WidgetSize::Small => BatteryEntry::Devices(vec![laptop]),
            WidgetSize::Medium | WidgetSize::Large => BatteryEntry::Devices(vec![
                laptop,
                cell("Phone", Device::Phone, 640),
                cell("Headphones", Device::Headphones, 450),
                cell("Mouse", Device::Mouse, 900),
            ]),
        }
    }

    fn view(entry: &BatteryEntry, cx: WidgetContext<NoIntent>) -> Element {
        match entry {
            BatteryEntry::Waiting => waiting(BatteryLayout::of(cx.size, MAX_RINGS)),
            BatteryEntry::Absent(words) => rsx! {
                div { class: "ds-batteries", "data-layout": "absent",
                    span { class: "ds-batteries-quiet", "{words}" }
                }
            },
            BatteryEntry::Devices(cells) => {
                let shown = &cells[..cells.len().min(MAX_RINGS)];
                match BatteryLayout::of(cx.size, shown.len()) {
                    BatteryLayout::Solo => solo(&shown[0]),
                    BatteryLayout::Grid => grid(shown),
                    BatteryLayout::Row => row(shown),
                }
            }
        }
    }
}

/// One battery's ring, its device's glyph inside.
fn ring(cell: &BatteryCell) -> Element {
    rsx! {
        BatteryRing { key: "{cell.name}", state: cell.state, label: cell.name.clone(),
            DeviceGlyph { device: cell.device, size: IconSize::Base }
        }
    }
}

/// The percentage, set in the widget's own size and face (the wrapper's; the label inherits).
fn figure(cell: &BatteryCell) -> Element {
    rsx! {
        Label { text: percent_text(cell.state.level), style: LabelStyle::Display }
    }
}

/// An empty place: the bare track.
fn empty(key: usize) -> Element {
    rsx! {
        BatteryRing { key: "empty-{key}", state: BatteryState::default(), label: "" }
    }
}

fn solo(cell: &BatteryCell) -> Element {
    rsx! {
        div { class: "ds-batteries", "data-layout": BatteryLayout::Solo.slug(),
            {ring(cell)}
            span { class: "ds-batteries-hero", {figure(cell)} }
        }
    }
}

fn grid(cells: &[BatteryCell]) -> Element {
    rsx! {
        div { class: "ds-batteries", "data-layout": BatteryLayout::Grid.slug(),
            for cell in cells {
                {ring(cell)}
            }
            for place in cells.len()..MAX_RINGS {
                {empty(place)}
            }
        }
    }
}

/// The Medium row: four places at the reference's fixed 80 pitch (M15), the batteries in the
/// first and a bare track, with no number, in each place left over, so one battery sits at the
/// left of the row rather than alone in the middle, and two never spread to the card's ends.
fn row(cells: &[BatteryCell]) -> Element {
    rsx! {
        div { class: "ds-batteries", "data-layout": BatteryLayout::Row.slug(),
            for cell in cells {
                div { key: "{cell.name}", class: "ds-batteries-cell",
                    {ring(cell)}
                    span { class: "ds-batteries-figure", {figure(cell)} }
                }
            }
            for place in cells.len()..MAX_RINGS {
                {empty_cell(place)}
            }
        }
    }
}

/// A row's place with no battery: the bare track over an empty figure line, so it keeps the
/// pitch and the height of a place with one.
fn empty_cell(place: usize) -> Element {
    rsx! {
        div { key: "empty-{place}", class: "ds-batteries-cell", "data-place": "empty",
            {empty(place)}
            span { class: "ds-batteries-figure" }
        }
    }
}

/// No reading yet: four bare tracks in the size's layout, no numbers.
fn waiting(layout: BatteryLayout) -> Element {
    rsx! {
        div { class: "ds-batteries", "data-layout": layout.slug(), "data-waiting": "",
            for place in 0..MAX_RINGS {
                {
                    match layout {
                        BatteryLayout::Row => empty_cell(place),
                        BatteryLayout::Solo | BatteryLayout::Grid => empty(place),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BatteryLayout;
    use crate::widget::kind::WidgetSize;

    #[test]
    fn the_layout_follows_the_size_and_the_count() {
        let cases = [
            (WidgetSize::Small, 1, BatteryLayout::Solo),
            (WidgetSize::Small, 2, BatteryLayout::Grid),
            (WidgetSize::Small, 4, BatteryLayout::Grid),
            (WidgetSize::Medium, 1, BatteryLayout::Row),
            (WidgetSize::Medium, 4, BatteryLayout::Row),
        ];
        for (size, count, want) in cases {
            assert_eq!(BatteryLayout::of(size, count), want, "{size:?} {count}");
        }
    }
}
