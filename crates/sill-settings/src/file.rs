//! `sill/settings.toml`, whole, and reading and writing it (design/22 §2): lenient read through
//! `ds_settings::lenient` (a bad value costs its own key), atomic temp-and-rename write. A key
//! this build does not know is ignored and reported on stderr ([`unknown_keys`]); a save drops
//! it.

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
    pub hot_corners: HotCornersSettings,
    /// sill's own idle service.
    pub idle: IdleSettings,
}

/// The stored settings in `dir`, or the defaults when there are none or they cannot be read.
pub fn load(dir: &Path) -> ShellFile {
    std::fs::read_to_string(dir.join(FILE_NAME))
        .map(|text| read(&text))
        .unwrap_or_default()
}

/// A dotted key path in the file that [`ShellFile`] has no field for (`dock.puppy`, `later`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnknownKey(pub String);

/// Every key in `given` that [`ShellFile`] does not know, depth first. A table sill knows is
/// walked; a key under a known non-table value (an array, a word) is that value's business.
pub fn unknown_keys(given: &toml::Table) -> Vec<UnknownKey> {
    match toml::Value::try_from(ShellFile::default()) {
        Ok(toml::Value::Table(known)) => unknown_under(given, &known, ""),
        _ => Vec::new(),
    }
}

fn unknown_under(given: &toml::Table, known: &toml::Table, prefix: &str) -> Vec<UnknownKey> {
    given
        .iter()
        .flat_map(|(key, value)| {
            let path = match prefix {
                "" => key.clone(),
                _ => format!("{prefix}.{key}"),
            };
            match (known.get(key), value) {
                (None, _) => vec![UnknownKey(path)],
                (Some(toml::Value::Table(inner)), toml::Value::Table(value)) => {
                    unknown_under(value, inner, &path)
                }
                (Some(_), _) => Vec::new(),
            }
        })
        .collect()
}

/// `text` read leniently, each unknown key reported once on stderr.
fn read(text: &str) -> ShellFile {
    if let Ok(given) = text.parse::<toml::Table>() {
        for UnknownKey(path) in unknown_keys(&given) {
            eprintln!("sill: {FILE_NAME}: unknown key `{path}` ignored");
        }
    }
    ds_settings::lenient(text)
}

/// What reading `dir`'s `settings.toml` for a live re-read produced: a
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
            Ok(_) => LoadOutcome::Read(Box::new(read(&text))),
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
