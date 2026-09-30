//! What the surface that opens a popup hands it: the scheme its root resolves and the room the
//! popup's surface leaves around the menu. A popup is a document of its own and starts no
//! second settings watch; the opener provides both as root contexts.

use ds::prelude::*;
use ds::style::material::stack::MaterialStack;
use ds::style::tokens::shell_type::ShellMetrics;

/// How a popup's `Ds` root resolves its scheme: the opener's own, handed over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupLook {
    pub appearance: Appearance,
    pub system: SystemPrefs,
    pub tint: Alpha,
    /// The shell type scale, written around the popup's root (its menu's 22 px rows).
    pub metrics: ShellMetrics,
    /// The material stack its root hands the menu card.
    pub stack: MaterialStack,
}

/// The transparent room, in logical pixels, between the popup surface's edges and the menu
/// card: the card's shadow lives there, and the surface clips at its edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupRoom {
    pub side: i32,
    pub top: i32,
    pub bottom: i32,
}

impl PopupRoom {
    /// The frame's inline padding: the room itself.
    pub fn style(self) -> String {
        format!(
            "padding:{}px {}px {}px {}px",
            self.top, self.side, self.bottom, self.side
        )
    }
}
