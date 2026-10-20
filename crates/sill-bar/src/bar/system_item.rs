//! The system menu's title, the bar's first (macOS's Apple menu): a glyph that opens
//! `BarMenu::System` whichever app is in front, so the session's actions are always one click away.

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;

use super::menus::BarMenu;
use super::status_item::StatusIcon;

/// The system menu's glyph. quire has no mark of the desktop's own yet; ⌘ stands in for it.
const GLYPH: Icon = Icon::Command;

/// The system menu's title on `output`.
#[component]
pub fn SystemItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    rsx! {
        StatusIcon {
            menu: BarMenu::System,
            icon: GLYPH,
            tooltip: "System".to_owned(),
        }
    }
}
