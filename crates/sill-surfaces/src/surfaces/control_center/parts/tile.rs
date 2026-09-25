//! Q78's stand-in: a control center module tile (design/13 §13.3.7: `--r-tile` 12, a glyph, a
//! title, a status line, an on/off state and an optional chevron to a detail pane), and the same
//! box spanning the row around the level, Now Playing, Appearance and Battery modules.
//!
//! quire's `ModuleTile` (on `cc-parts`) draws the glyph on an accent disc when on; this one has
//! no disc (a consumer does not paint one: rule 3), so the on state is the quire `Toggle` in the
//! tile's corner and the glyph's accent ink.

use dioxus::prelude::*;
use ds::{Availability, Glyph, Icon, IconButton, IconButtonVariant, Switch, Toggle};

/// Whether a tile's feature is on (quire's `ModuleState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum ModuleState {
    #[default]
    Off,
    On,
}

impl ModuleState {
    fn switch(self) -> Switch {
        match self {
            ModuleState::Off => Switch::Off,
            ModuleState::On => Switch::On,
        }
    }

    fn of(switch: Switch) -> ModuleState {
        match switch {
            Switch::Off => ModuleState::Off,
            Switch::On => ModuleState::On,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            ModuleState::Off => "off",
            ModuleState::On => "on",
        }
    }
}

/// How much of the grid's row a tile takes (quire's `TileSpan`; the grid's own
/// `geometry::Span` decides it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TileSpan {
    Half,
    Full,
}

impl TileSpan {
    fn slug(self) -> &'static str {
        match self {
            TileSpan::Half => "half",
            TileSpan::Full => "full",
        }
    }
}

/// An on/off module: `glyph` and the switch, then `title` over `status`, and a chevron to its
/// detail pane when `detail` is given. `Disabled` shows the state and takes nothing.
#[component]
pub(crate) fn ModuleTile(
    glyph: Icon,
    title: String,
    status: String,
    state: ModuleState,
    on_toggle: EventHandler<ModuleState>,
    #[props(default)] detail: Option<EventHandler<()>>,
    #[props(default)] availability: Availability,
) -> Element {
    let chevron = detail.map(|open| {
        rsx! {
            IconButton {
                variant: IconButtonVariant::Tool,
                icon: Icon::ChevronRight,
                label: format!("{title} details"),
                onclick: move |_| open.call(()),
            }
        }
    });
    rsx! {
        div {
            class: "sill-cc-tile",
            "data-span": TileSpan::Half.slug(),
            "data-state": state.slug(),
            div { class: "sill-cc-tile-head",
                span { class: "sill-cc-glyph", Glyph { icon: glyph } }
                Toggle {
                    label: title.clone(),
                    value: state.switch(),
                    availability,
                    onchange: move |to: Switch| on_toggle.call(ModuleState::of(to)),
                }
            }
            div { class: "sill-cc-tile-foot",
                div { class: "sill-cc-tile-text",
                    span { class: "sill-cc-title", "{title}" }
                    span { class: "sill-cc-status ds-truncate", "{status}" }
                }
                {chevron}
            }
        }
    }
}

/// The tile's box around a module that draws its own content (a level, Now Playing, the
/// appearance picker, the battery).
#[component]
pub(crate) fn TileFrame(span: TileSpan, children: Element) -> Element {
    rsx! {
        div { class: "sill-cc-tile", "data-span": span.slug(), {children} }
    }
}
