//! `sill/settings.toml`, whole (design/22 §2): a [`SettingsDoc`] that `ds_settings::Store` reads
//! leniently (a bad value costs its own key), writes atomically and watches. A key this build
//! does not know is reported by the load and dropped by the next save; `sill::daemon::settings`
//! owns the reading, saving and watching.

use ds_settings::{AppName, FileName, Format, SettingsDoc};
use serde::{Deserialize, Serialize};

use crate::bar::BarSettings;
use crate::calendar::CalendarSettings;
use crate::control_center::ControlCenterSettings;
use crate::display::DisplaySettings;
use crate::dock::DockSettings;
use crate::hot_corners::HotCornerSettings;
use crate::idle::IdleSettings;
use crate::keyboard::KeyboardSettings;
use crate::launcher::LauncherSettings;
use crate::menus::MenusSettings;
use crate::notifications::NotificationsSettings;
use crate::osd::OsdSettings;
use crate::power_menu::PowerMenuSettings;
use crate::screenshot::ScreenshotSettings;
use crate::scroll::ScrollSettings;
use crate::scrollbar::ScrollbarSettings;
use crate::session::SessionSettings;
use crate::spaces::SpacesSettings;
use crate::switcher::SwitcherSettings;
use crate::widgets::WidgetsSettings;
use crate::window::WindowSettings;

/// The directory name under `$XDG_CONFIG_HOME`.
pub const APP: AppName = AppName("sill");

/// Every shell domain (design/22 §4.4 `ShellFile`, with `switcher` as a table of its own).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellFile {
    pub bar: BarSettings,
    pub dock: DockSettings,
    pub launcher: LauncherSettings,
    pub scroll: ScrollSettings,
    pub scrollbar: ScrollbarSettings,
    pub menus: MenusSettings,
    pub switcher: SwitcherSettings,
    pub notifications: NotificationsSettings,
    pub control_center: ControlCenterSettings,
    pub spaces: SpacesSettings,
    pub display: DisplaySettings,
    pub osd: OsdSettings,
    pub window: WindowSettings,
    pub power_menu: PowerMenuSettings,
    /// sill's own lock screen.
    pub session: SessionSettings,
    pub widgets: WidgetsSettings,
    pub calendar: CalendarSettings,
    pub screenshot: ScreenshotSettings,
    pub hot_corners: HotCornerSettings,
    /// sill's own idle service.
    pub idle: IdleSettings,
    /// How the keyboard is treated per app (keycap profiles).
    pub keyboard: KeyboardSettings,
}

impl SettingsDoc for ShellFile {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}
