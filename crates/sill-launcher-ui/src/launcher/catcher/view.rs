//! The catcher (design/20 §1.3): a transparent Overlay over the whole output, shown with the
//! panel, painted once per showing and never animated or dimmed (plan "shell risks"), whose
//! press anywhere outside the panel closes the launcher. Its input region leaves out what the
//! panel takes (its content-sized card, or the whole panel while the actions menu is open or
//! before the card is measured; `hit::PanelHit`), so a press on the panel reaches the panel
//! whichever surface is on top and a press under a short card closes the launcher.

use dioxus::prelude::*;
use ds::prelude::AppStyle;
use shell_host::{OutputInfo, use_watch};

use super::layer::input;
use crate::launcher::card::follow_input;
use crate::launcher::follow::{use_follow_visibility, use_shown};
use crate::launcher::panel::LAUNCHER_CSS;
use crate::launcher::tokens::LauncherTokens;
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::root::use_root_style;

/// The catcher on `output`.
pub(super) fn catcher(output: OutputInfo) -> Element {
    let link = use_context::<LauncherLink>();
    let shown = use_shown(&link);
    use_follow_visibility(shown, output.id);
    let hit = use_watch(link.subscribe_hit());
    let place = use_watch(link.subscribe_place());
    let tokens = place().map_or_else(LauncherTokens::default, LauncherTokens::from);
    follow_input(input(&output, &tokens, hit()));
    let surface = use_root_style().surface;
    rsx! {
        AppStyle { css: LAUNCHER_CSS }
        div {
            class: "sill-launcher-catcher",
            "data-surface": surface,
            onmousedown: move |_| link.close(),
        }
    }
}
