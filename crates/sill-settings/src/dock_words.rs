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
    /// `dock.bounce`: gates Informational and Critical bounces.
    Bounce { #[default] On, Off }
    /// `dock.launch_animation`: gates the launch bounce.
    LaunchAnim { #[default] On, Off }
    /// `dock.click_active_app`: `Nothing` is macOS.
    ActiveClick { #[default] Cycle, Nothing }
    /// `dock.trash`.
    TrashTile { #[default] On, Off }
    /// `dock.context_menu_order`.
    DockMenuOrder { #[default] WindowsFirst, AppleOrder }
    /// `dock.modifier_clicks`.
    DockModifierClicks { #[default] AppleMapping, Off }
    /// `dock.floor`: the reflective band along the pill's floor, under the tiles.
    DockFloor { #[default] Off, On }
}
