//! A detail pane's foot: "Wi-Fi Settings…" or "Bluetooth Settings…", which opens
//! Settings (joining a new network or pairing a device is done there) and closes
//! the panel.

use dioxus::prelude::*;
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use shell_host::SurfaceHandle;
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::LaunchCommand;
use sill_services::LaunchService;
use sill_services::control::Control;
use sill_shell_kit::prelude::*;

use sill_ui_kit::hooks::use_control;
use sill_ui_kit::surface::launch_with_token;

/// The app the settings run as.
const SETTINGS_APP: &str = "org.quire.Settings";

/// How a pane closes the popup it is in: provided by the control center and by a module's bar
/// dropdown.
#[derive(Clone, Copy)]
pub(crate) struct ClosePanel(pub Callback<()>);

/// What opens Settings from a pane: the launch service, the popup's surface (for the activation
/// token) and how to close the popup.
#[derive(Clone)]
pub(crate) struct SettingsOpener {
    launch: Control<LaunchCommand>,
    surface: Option<SurfaceHandle>,
    close: Option<ClosePanel>,
}

impl SettingsOpener {
    /// Launch Settings with a token from the still-mapped popup, then close the popup.
    pub(crate) fn open(&self) {
        let opener = self.clone();
        spawn(async move {
            // A stopped launcher drops it; the panel closes either way.
            let _gone = launch_with_token(
                opener.surface.as_ref(),
                &opener.launch,
                AppId(SETTINGS_APP.to_owned()),
                DesktopAction::Default,
                Vec::new(),
            )
            .await;
            if let Some(ClosePanel(close)) = opener.close {
                close.call(());
            }
        });
    }
}

/// The opener for the pane being built.
pub(crate) fn use_settings_opener() -> SettingsOpener {
    SettingsOpener {
        launch: use_control::<LaunchService>(),
        surface: try_use_context::<SurfaceHandle>(),
        close: try_use_context::<ClosePanel>(),
    }
}

/// The foot: `label` opening Settings.
#[component]
pub(crate) fn SettingsFoot(label: String) -> Element {
    let opener = use_settings_opener();
    rsx! {
        PaneFooter {
            Button {
                bezel: Bezel::Inline,
                label,
                onclick: move |_| opener.open(),
            }
        }
    }
}
