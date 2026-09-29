//! A detail pane's foot: "Wi-Fi Settings…" or "Bluetooth Settings…", which opens COSMIC
//! Settings (joining a new network or pairing a device is done there) and closes
//! the panel; and the line a pane shows in place of an empty list.

use dioxus::prelude::*;
use ds::{Button, ButtonVariant};
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_services::LaunchService;

use crate::hooks::use_control;

/// The app COSMIC's settings run as.
const SETTINGS_APP: &str = "com.system76.CosmicSettings";

/// How a pane closes the popup it is in: provided by the control center and by a module's bar
/// dropdown.
#[derive(Clone, Copy)]
pub(crate) struct ClosePanel(pub Callback<()>);

/// The foot: a rule, then `label` opening COSMIC Settings.
#[component]
pub(crate) fn SettingsFoot(label: String) -> Element {
    let launch = use_control::<LaunchService>();
    let close = try_use_context::<ClosePanel>();
    rsx! {
        div { class: "sill-cc-detail-foot",
            Button {
                variant: ButtonVariant::Quiet,
                label,
                onclick: move |_| {
                    // A stopped launcher drops it; the panel closes either way.
                    let _gone = launch.send(open_settings());
                    if let Some(ClosePanel(close)) = close {
                        close.call(());
                    }
                },
            }
        }
    }
}

/// The line in place of an empty list: "Wi-Fi is off", "No devices".
#[component]
pub(crate) fn EmptyLine(text: String) -> Element {
    rsx! {
        div { class: "sill-cc-empty", "{text}" }
    }
}

/// Launch COSMIC Settings.
pub(crate) fn open_settings() -> LaunchCommand {
    LaunchCommand::Launch(LaunchRequest {
        app: AppId(SETTINGS_APP.to_owned()),
        action: DesktopAction::Default,
        files: Vec::new(),
        uris: Vec::new(),
        token: None,
    })
}
