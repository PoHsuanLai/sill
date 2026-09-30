//! The catcher (design/20 §1.3): a transparent Overlay over the whole output, shown with the
//! panel, painted once per showing and never animated or dimmed (plan "shell risks"), whose
//! press anywhere outside the panel closes the launcher. Its input region leaves out what the
//! panel takes (its content-sized card, or the whole panel while the actions menu is open or
//! before the card is measured; `hit::PanelHit`), so a press on the panel reaches the panel
//! whichever surface is on top and a press under a short card closes the launcher.

use dioxus::prelude::*;
use shell_host::{OutputInfo, use_watch};

use super::layer::input;
use crate::launcher::card::follow_input;
use crate::launcher::follow::{use_follow_visibility, use_shown};
use crate::launcher::panel::LAUNCHER_CSS;
use sill_ui_kit::hooks::use_settings;
use sill_ui_kit::links::LauncherLink;

/// The catcher on `output`.
pub(super) fn catcher(output: OutputInfo) -> Element {
    let link = use_context::<LauncherLink>();
    let shown = use_shown(&link);
    use_follow_visibility(shown, output.id);
    let file = use_settings()();
    let hit = use_watch(link.subscribe_hit());
    follow_input(input(&output, &file.bar, &file.launcher, hit()));
    rsx! {
        style { {LAUNCHER_CSS} }
        div {
            class: "sill-launcher-catcher",
            onmousedown: move |_| link.close(),
        }
    }
}
