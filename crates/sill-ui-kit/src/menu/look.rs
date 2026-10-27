//! What the surface that opens a popup hands it: the scheme its root resolves and the room the
//! popup's surface leaves around the menu. A popup is a document of its own and starts no
//! second settings watch; the opener provides both as root contexts.

use ds::prelude::*;
use ds::style::material::stack::MaterialStack;

use crate::hooks::use_appearance;

/// How a popup's `Ds` root resolves its scheme: the opener's own, handed over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupLook {
    pub appearance: Appearance,
    pub system: SystemPrefs,
    pub tint: Alpha,
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

/// A Context menu's width: quire's `menu.css` fixes Slim and Context at 220.
pub const CONTEXT_MENU_WIDTH: i32 = 220;

impl PopupRoom {
    /// The room around a context menu, for `--shadow-pop` (its ambient layer reaches 28 px).
    pub const CONTEXT: PopupRoom = PopupRoom {
        side: 28,
        top: 8,
        bottom: 40,
    };

    /// The frame's inline padding: the room itself.
    pub fn style(self) -> String {
        format!(
            "padding:{}px {}px {}px {}px",
            self.top, self.side, self.bottom, self.side
        )
    }
}

/// The look a popup this surface opens is handed: the surface's own appearance, tint
/// and material stack, read live from the settings and the desktop's appearance.
pub fn use_popup_look() -> PopupLook {
    let environment = use_appearance()();
    PopupLook {
        appearance: environment.settings.appearance.appearance(),
        system: environment.system,
        tint: environment.tint_alpha(),
        stack: environment.material_stack(),
    }
}
