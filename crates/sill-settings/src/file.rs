//! `sill/settings.toml`, whole, and reading and writing it (design/22 §2): lenient read through
//! `ds_settings::lenient` (a bad value costs its own key), atomic temp-and-rename write,
//! unknown keys and tables preserved.

use std::path::{Path, PathBuf};

use ds_settings::{AppName, SettingsError};
use serde::{Deserialize, Serialize};

use crate::bar::BarSettings;
use crate::calendar::CalendarSettings;
use crate::control_center::ControlCenterSettings;
use crate::display::DisplaySettings;
use crate::dock::DockSettings;
use crate::hot_corners::HotCornersSettings;
use crate::idle::IdleSettings;
use crate::launcher::LauncherSettings;
use crate::menus::MenusSettings;
use crate::notifications::NotificationsSettings;
use crate::osd::OsdSettings;
use crate::power_menu::PowerMenuSettings;
use crate::screenshot::ScreenshotSettings;
use crate::scroll::ScrollSettings;
use crate::scrollbar::ScrollbarSettings;
use crate::session::SessionSettings;
use crate::sound::SoundSettings;
use crate::spaces::SpacesSettings;
use crate::switcher::SwitcherSettings;
use crate::widgets::WidgetsSettings;
use crate::window::WindowSettings;

/// The directory name under `$XDG_CONFIG_HOME` (and `$XDG_STATE_HOME`, `$XDG_CACHE_HOME`).
pub const APP: AppName = AppName("sill");

/// The file's name inside [`settings_dir`].
pub const FILE_NAME: &str = "settings.toml";

/// `$XDG_CONFIG_HOME/sill`, else `$HOME/.config/sill`.
pub fn settings_dir() -> Option<PathBuf> {
    ds_settings::config_dir(APP)
}

/// Every shell domain (design/22 §4.4 `ShellFile`, with `switcher` and `sound` as tables of
/// their own: FINDINGS.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellFile {
    /// 1. Nothing reads it yet but its absence.
    pub version: u16,
    pub bar: BarSettings,
    pub dock: DockSettings,
    pub launcher: LauncherSettings,
    pub scroll: ScrollSettings,
    pub scrollbar: ScrollbarSettings,
    pub menus: MenusSettings,
    pub switcher: SwitcherSettings,
    pub sound: SoundSettings,
    pub notifications: NotificationsSettings,
    pub control_center: ControlCenterSettings,
    pub spaces: SpacesSettings,
    pub display: DisplaySettings,
    pub osd: OsdSettings,
    pub window: WindowSettings,
    /// M5 (FINDINGS "M5 freeze"): no design/22 rows yet.
    pub power_menu: PowerMenuSettings,
    /// sill's own lock screen.
    pub session: SessionSettings,
    /// M10 (FINDINGS "M10 freeze"): no design/22 rows yet.
    pub widgets: WidgetsSettings,
    pub calendar: CalendarSettings,
    pub screenshot: ScreenshotSettings,
    pub hot_corners: HotCornersSettings,
    /// sill's own idle service (FINDINGS "sill idle (Q420 B)"): no design/22 rows yet.
    pub idle: IdleSettings,
    /// Tables this build does not know, kept for the next write.
    #[serde(flatten)]
    pub extra: toml::Table,
}

impl Default for ShellFile {
    fn default() -> Self {
        ShellFile {
            version: 1,
            bar: BarSettings::default(),
            dock: DockSettings::default(),
            launcher: LauncherSettings::default(),
            scroll: ScrollSettings::default(),
            scrollbar: ScrollbarSettings::default(),
            menus: MenusSettings::default(),
            switcher: SwitcherSettings::default(),
            sound: SoundSettings::default(),
            notifications: NotificationsSettings::default(),
            control_center: ControlCenterSettings::default(),
            spaces: SpacesSettings::default(),
            display: DisplaySettings::default(),
            osd: OsdSettings::default(),
            window: WindowSettings::default(),
            power_menu: PowerMenuSettings::default(),
            session: SessionSettings::default(),
            widgets: WidgetsSettings::default(),
            calendar: CalendarSettings::default(),
            screenshot: ScreenshotSettings::default(),
            hot_corners: HotCornersSettings::default(),
            idle: IdleSettings::default(),
            extra: toml::Table::new(),
        }
    }
}

/// The stored settings in `dir`, or the defaults when there are none or they cannot be read.
pub fn load(dir: &Path) -> ShellFile {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .map(|text| ds_settings::lenient(&text))
        .unwrap_or_default()
}

/// What reading `dir`'s `settings.toml` for a live re-read produced (FINDINGS F993, F1030): a
/// bad *value* inside a valid table costs only its own key ([`load`], `ds_settings::lenient`),
/// but text that is not TOML at all — a half-written file the watch's debounce did not fully
/// cover, or a hand edit with a syntax error — must not blank every setting to its default while
/// a caller can keep the last good file instead. A missing or unreadable file is still every
/// setting's default, as [`load`] treats it: nothing was ever good to keep.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadOutcome {
    /// The file parsed (its values leniently, key by key). Boxed: `Unparseable` carries nothing,
    /// and `ShellFile` is large (`clippy::large_enum_variant`).
    Read(Box<ShellFile>),
    /// The file exists but is not TOML at all.
    Unparseable,
}

/// [`LoadOutcome`] for `dir`'s `settings.toml`.
pub fn try_load(dir: &Path) -> LoadOutcome {
    match std::fs::read_to_string(dir.join(FILE_NAME)) {
        Ok(text) => match text.parse::<toml::Table>() {
            Ok(_) => LoadOutcome::Read(Box::new(ds_settings::lenient(&text))),
            Err(_) => LoadOutcome::Unparseable,
        },
        Err(_) => LoadOutcome::Read(Box::default()),
    }
}

/// Write `file` to `dir` (created if needed) through a temporary file and a rename, so a
/// directory watch sees one rename and a crash leaves no half-written file.
pub fn save(dir: &Path, file: &ShellFile) -> Result<(), SettingsError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| SettingsError::Io { path, source }
    };
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let path = dir.join(FILE_NAME);
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, toml::to_string(file)?).map_err(io(&tmp))?;
    std::fs::rename(&tmp, &path).map_err(io(&path))
}
