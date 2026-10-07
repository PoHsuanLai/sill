//! The words a `WidgetFrame` is described in (design/22-SETTINGS.md section 3.20):
//! its footprint on the grid, where it is drawn, and its title row.

use ds::base::word::Word;
use ds::components::content::text_runs::TextLine;
use ds::style::icon::Icon;
use serde::{Deserialize, Serialize};

/// A widget's footprint on the grid unit (`widgets.desktop_cell_px`, `desktop_gap_px`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WidgetSize {
    /// One cell.
    #[default]
    Small,
    /// Two cells wide, one tall.
    Medium,
    /// Two cells wide, two tall.
    Large,
}

/// Where a widget is drawn, which decides what its card is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WidgetHost {
    /// On the desktop layer: a card of the `Widget` material, over the wallpaper.
    #[default]
    Desktop,
    /// In the notification center's widget column: a flatter tile on the Popover it sits in,
    /// with no material of its own (a second material inside the center would draw a second
    /// edge and drop).
    Tile,
}

/// What a desktop widget's card is tinted with (design/23-WIDGETS.md section 4.3). Every card
/// takes the Space's tint (settled 2026-09-27, Arc's contribution); `Material` stays for a host
/// that must show the bare material (a comparison page).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum CardTint {
    /// The `Widget` material's own tint.
    Material,
    /// The material with the Space's gradient over it at the material's frame alpha, as the
    /// tinted chrome carries it (design/03-COLOR.md section 17.2), so the Space's colour reaches
    /// the card.
    #[default]
    Space,
}

impl CardTint {
    /// `data-tint`: written only for a Space tint.
    pub fn attr(self) -> Option<&'static str> {
        (self != CardTint::Material).then(|| self.slug())
    }

    /// The tint a card in `host` draws: a tile sits on the notification center's Popover, which
    /// already carries the Space, so it lays no second gradient.
    pub fn on(self, host: WidgetHost) -> CardTint {
        match host {
            WidgetHost::Desktop => self,
            WidgetHost::Tile => CardTint::Material,
        }
    }
}

/// Whether a widget's card is picked up (design/23 section 9.8): a host moving a
/// desktop widget, or the widget gallery holding one, lifts it; quire owns the scale
/// (`--pickup`), the shadow (`--shadow-drag`, in place of the resting drop), the layer
/// (`--z-drag`) and the timing (`--t-quick` at `--e-out`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum Lift {
    /// Where it lies.
    #[default]
    Rest,
    /// Picked up.
    Lifted,
}

impl Lift {
    /// `data-lift`: written only while lifted.
    pub fn attr(self) -> Option<&'static str> {
        (self != Lift::Rest).then(|| self.slug())
    }
}

/// A widget's title row: a glyph and a name ("Batteries", a calendar's month).
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetTitle {
    /// The glyph before the name.
    pub glyph: Icon,
    /// The name.
    pub text: TextLine,
}

impl WidgetTitle {
    /// A title of `glyph` and `text`.
    pub fn new(glyph: Icon, text: impl Into<TextLine>) -> Self {
        WidgetTitle {
            glyph,
            text: text.into(),
        }
    }
}
