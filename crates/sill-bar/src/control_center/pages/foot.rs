//! A detail pane's foot: "Wi-Fi Settings…" or "Bluetooth Settings…", which opens
//! Settings (joining a new network or pairing a device is done there) and closes
//! the panel.

use dioxus::prelude::*;
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use ds_settings::schema::PageId;
use ds_settings::target::SettingsTarget;
use sill_launcher::SettingsLink;
use sill_launcher::actions::ActionId;
use sill_model::actions::Subject;
use sill_shell_kit::prelude::*;

use sill_ui_kit::hooks::{Entrance, use_entrance, use_link};

/// How a pane closes the popup it is in: provided by the control center and by a module's bar
/// dropdown.
#[derive(Clone, Copy)]
pub(crate) struct ClosePanel(pub Callback<()>);

/// What opens Settings from a pane: the shell's Open Settings action, asked from the bar the
/// popup hangs from (for the activation token), and how to close the popup.
#[derive(Clone)]
pub(crate) struct SettingsOpener {
    entrance: Entrance,
    close: Option<ClosePanel>,
}

impl SettingsOpener {
    /// Open Settings at `link`, then close the popup.
    pub(crate) fn open(&self, link: SettingsLink) {
        self.entrance
            .do_to(ActionId::OpenSettings, Subject::Settings(link));
        if let Some(ClosePanel(close)) = self.close {
            close.call(());
        }
    }
}

/// The opener for the pane being built.
pub(crate) fn use_settings_opener() -> SettingsOpener {
    SettingsOpener {
        entrance: use_entrance(),
        close: use_link::<ClosePanel>(),
    }
}

/// The link to the Settings page `id` (`wifi`, `bluetooth`).
pub(crate) fn page_link(id: &str) -> SettingsLink {
    SettingsLink(SettingsTarget::Page(PageId::new(id)).to_string())
}

/// The foot: `label` opening Settings at the page `page`.
#[component]
pub(crate) fn SettingsFoot(label: String, page: &'static str) -> Element {
    let opener = use_settings_opener();
    rsx! {
        PaneFooter {
            Button {
                bezel: Bezel::Inline,
                label,
                onclick: move |_| opener.open(page_link(page)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::page_link;

    #[test]
    fn a_foot_links_to_its_page_by_id() {
        assert_eq!(page_link("wifi").0, "page:wifi");
        assert_eq!(page_link("bluetooth").0, "page:bluetooth");
    }
}
