//! The dock's closed choices (design/22 §4.1): every two-state key is an enum with named
//! variants, never a `bool`.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

macro_rules! words {
    ($(#[$doc:meta] $name:ident { #[default] $first:ident $(, $rest:ident)* $(,)? })*) => {$(
        #[$doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word,
        )]
        #[serde(rename_all = "snake_case")]
        #[word(case = snake)]
        pub enum $name {
            #[default]
            $first,
            $($rest,)*
        }
    )*};
}

words! {
    /// `dock.magnification`.
    Magnification { #[default] On, Off }
    /// `dock.autohide`.
    AutoHide { #[default] Off, On }
    /// `dock.indicators`.
    Indicators { #[default] On, Off }
    /// `dock.click_active_app`: `Nothing` is macOS.
    ActiveClick { #[default] Cycle, Nothing }
    /// `dock.trash`.
    TrashTile { #[default] On, Off }
    /// `dock.floor`: the reflective band along the pill's floor, under the tiles.
    DockFloor { #[default] Off, On }
}

/// `dock.context_menu_order`. The words stay as stored; the labels are what Settings shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum DockMenuOrder {
    /// The Mac's order: options, then the app's own items, then Quit.
    #[default]
    #[word(label = "Standard")]
    AppleOrder,
    /// The open windows first, flat.
    WindowsFirst,
}

/// `dock.modifier_clicks`. The words stay as stored; the labels are what Settings shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum DockModifierClicks {
    /// The Mac's clicks: Ctrl shows in Files, Alt switches and hides.
    #[default]
    #[word(label = "Standard")]
    AppleMapping,
    /// Modifier keys change nothing.
    Off,
}
