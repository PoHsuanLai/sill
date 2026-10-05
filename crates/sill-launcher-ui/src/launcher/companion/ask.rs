//! What a prompt sent from the panel carries: the chips the person kept, the window they asked
//! over and its app. The panel's chips are the window it was summoned over, when there was one;
//! the launcher has no results or selection of its own to offer.

use docket_core::{ContextKeep, Keep};
use prov::AppName;
use sill_launcher::WindowKey;
use sill_model::companion::LauncherOrigin;
use sill_model::companion::{AskInput, Summon};
use sill_model::cosmic_wl::Toplevels;

/// The ask for `prompt` in the summon `summon` found the panel in, over `toplevels`.
pub fn ask_input(prompt: String, summon: &Summon, toplevels: &Toplevels) -> AskInput {
    let window = match summon {
        Summon::InLauncher {
            origin: LauncherOrigin::Fresh { window },
        } => *window,
        Summon::Rest
        | Summon::AskingApp { .. }
        | Summon::InApp { .. }
        | Summon::InLauncher {
            origin: LauncherOrigin::FromSearch,
        } => None,
    };
    AskInput {
        prompt,
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: match window {
                Some(_) => Keep::Kept,
                None => Keep::Dropped,
            },
        },
        window,
        app: window.and_then(|key| app_of(toplevels, key)),
    }
}

/// The app of a window, when its Wayland app id is a bus name.
fn app_of(toplevels: &Toplevels, key: WindowKey) -> Option<AppName> {
    toplevels
        .windows
        .iter()
        .find(|window| window.key == key)
        .and_then(|window| AppName::parse(&window.app_id).ok())
}
