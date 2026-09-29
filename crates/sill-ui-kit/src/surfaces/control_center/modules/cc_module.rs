//! What a control center module is: the one trait every module implements, and the context a
//! module is drawn in. The grid tile, the bar item's dropdown, the detail page and the bar item
//! itself are the module's own; `super::module` is the only place that names a
//! [`ControlCenterModule`] to pick one.

use dioxus::prelude::*;
use sill_model::ids::OutputName;
use sill_settings::{ControlCenterSettings, InMenuBar};

use crate::surfaces::bar::BarMenu;
use crate::surfaces::control_center::page::Detail;

/// How many grid columns a module takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Span {
    /// One column: a square-ish tile (Wi-Fi, Bluetooth, Focus).
    One,
    /// The whole row: sliders (Display, Sound), Now Playing, Appearance, Battery.
    Row,
}

/// What a module is drawn with: the bar's output (the Display module's brightness target), the
/// chevron that opens a detail page, and the back button of a detail page (none in a bar item's
/// dropdown, which has nothing to go back to).
#[derive(Clone)]
pub(crate) struct CcCx {
    pub output: Option<OutputName>,
    pub on_detail: EventHandler<Detail>,
    pub on_back: Option<EventHandler<()>>,
}

/// A module's bar item: the menu its click opens and the icon it draws.
#[derive(Clone, Copy)]
pub(crate) struct BarItemFace {
    pub menu: BarMenu,
    /// The item's component, reading its own services.
    pub item: fn() -> Element,
}

/// One control center module. Wi-Fi, Bluetooth, Focus, Display, Keyboard, Sound, Now Playing,
/// Appearance and Battery implement it; a module that has no detail page, no bar item or no
/// dropdown keeps the provided answer.
pub(crate) trait CcModule: Sync {
    /// The columns the tile takes.
    fn span(&self) -> Span;

    /// The grid tile.
    fn tile(&self, cx: &CcCx) -> Element;

    /// The body of the module's bar-item dropdown: its tile on a bare plate, unless the module
    /// has a pane of its own. `None` for a module that is not a bar item.
    fn dropdown(&self, cx: &CcCx) -> Option<Element> {
        self.bar_item().map(|_| self.tile(cx))
    }

    /// Whether `settings` show the module as a bar item of its own (`control_center.menu_bar_*`).
    fn menu_bar(&self, _settings: &ControlCenterSettings) -> InMenuBar {
        InMenuBar::Hide
    }

    /// The module's bar item; `None` for a module that cannot be one.
    fn bar_item(&self) -> Option<BarItemFace> {
        None
    }

    /// The detail page `page` when it is this module's, sliding in over the grid.
    fn detail(&self, _page: Detail, _cx: &CcCx) -> Option<Element> {
        None
    }
}
