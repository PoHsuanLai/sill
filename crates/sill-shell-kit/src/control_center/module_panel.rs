//! ModulePanel: a control-center module that holds content of its own (the Sound and Display
//! levels, Now Playing, Appearance, Battery) on a `ModuleTile`'s frame, so a consumer draws no
//! plate of its own (design/13-BEHAVIOUR-menus-windows.md section 13.3.7).
//!
//! A tile is a toggle with a glyph, a title and a status; a panel is not pressable at all: its
//! content (a `Slider`, a picker, buttons) takes every press, so the panel is a plain
//! `div` with no role. It spans the whole grid row by default, as a slider module does.

use crate::control_center::module_tile_kind::TileSpan;
use dioxus::prelude::*;
use ds::base::vocab::Availability;
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::content::text_runs::{TextLine, text};
use ds::root::common::Common;
use ds::style::icon::render::IconSize;

/// Whether the panel paints the tile's plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum PanelPlate {
    /// The tile's plate: `--surface-2`, a hairline, `--r-tile`, the module's padding.
    #[default]
    Tile,
    /// No plate and no padding: the module where something else already is its plate (a bar
    /// item's dropdown, whose popover card is the module's).
    Bare,
}

/// A module with content: an optional header row (`glyph`, `title`, then `trailing` at the far
/// end, such as a level's percentage) over `children`. The header is drawn when any of the three
/// is given.
///
/// `glyph` is an `Icon` (it converts) or any [`IconSource`]: `IconSource::Status` draws a layered
/// status glyph (the Battery module's). `availability` is its content's: a module whose level is disabled
/// passes `Availability::Disabled` and its header glyph and trailing figure dim with it.
#[component]
pub fn ModulePanel(
    #[props(default)] glyph: Option<IconSource>,
    #[props(default)] title: Option<TextLine>,
    #[props(default)] trailing: Option<Element>,
    #[props(default = TileSpan::Full)] span: TileSpan,
    #[props(default)] plate: PanelPlate,
    #[props(default)] availability: Availability,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    let head = header(glyph, title, trailing);
    let class = common.class("ds-module-panel");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            "data-span": span.slug(),
            "data-plate": plate.slug(),
            "data-availability": availability.slug(),
            "aria-label": common.aria_label.clone(),
            "aria-disabled": availability.aria_disabled(),
            onmounted: move |event| common.mounted(event),
            ..data,
            {head}
            {children}
        }
    }
}

/// The header row, or nothing when the panel has neither glyph, title nor trailing slot.
fn header(
    glyph: Option<IconSource>,
    title: Option<TextLine>,
    trailing: Option<Element>,
) -> Element {
    if glyph.is_none() && title.is_none() && trailing.is_none() {
        return rsx! {};
    }
    rsx! {
        div { class: "ds-module-panel-head",
            if let Some(source) = glyph {
                span { class: "ds-module-panel-glyph",
                    IconView { source, size: IconSize::Base }
                }
            }
            span { class: "ds-module-panel-title",
                if let Some(title) = title {
                    {text(&title)}
                }
            }
            if let Some(trailing) = trailing {
                span { class: "ds-module-panel-trailing", {trailing} }
            }
        }
    }
}
