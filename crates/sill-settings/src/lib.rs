//! `$XDG_CONFIG_HOME/sill/settings.toml`: the shell's settings domains (design/22-SETTINGS.md
//! §3.4-3.8 and §3.11-3.14: bar, dock, launcher, scroll, scrollbar, menus, sound,
//! notifications, control center, spaces; `display`, `osd`, `power_menu` and `session`; and
//! `calendar`, `screenshot` and `hot_corners`; §3.24-3.25: `idle` and `keyboard`; §3.30: `companion`; `voice` and `wallpaper`, which design/22 does not list yet), and their schema (§9,
//! `#[derive(SettingsSchema)]` with `file = "sill/settings.toml"`), all fields of a [`ShellFile`].
//!
//! Pure data: no I/O, no runtime, no dioxus. `ShellFile` is a `ds_settings::SettingsDoc`, and
//! reading, saving, watching and publishing it belong to the daemon (`sill::daemon::settings`).

pub mod bar;
pub mod calendar;
pub mod companion;
pub mod control_center;
pub mod display;
pub mod dock;
mod dock_words;
pub mod file;
pub mod focus;
pub mod hot_corners;
pub mod idle;
pub mod keyboard;
pub mod launcher;
pub mod launcher_kinds;
pub mod notifications;
pub mod osd;
pub mod schema;
pub mod screenshot;
pub mod scroll;
pub mod scrollbar;
pub mod session;
pub mod voice;
pub mod wallpaper;
pub mod widgets;

pub use bar::{BarSettings, MenuBarHiding};
pub use calendar::{CalendarSettings, FirstWeekday, WeekNumbers};
pub use companion::{
    CONFIRM_ARM, CompanionSettings, InField, NotifyWhen, SummonKey, confirm_arm,
    params as companion_params,
};
pub use control_center::{ControlCenterModule, ControlCenterSettings, InMenuBar};
pub use display::{DisplaySettings, NightWarmthMode};
pub use dock::{AutoHide, DockFloor, DockSettings, Indicators, Magnification, TrashTile};
pub use file::{APP, ShellFile};
pub use focus::{FocusMode, focus_mode, with_focus_mode};
pub use hot_corners::{
    CornerAction, CornerBinding, CornerBindings, CornerDo, CornerKey, CornerKeys,
    HotCornerSettings, OpenItem,
};
pub use idle::IdleSettings;
pub use keyboard::{KeyboardOverride, KeyboardSettings, RejectedRow, overrides};
pub use launcher::LauncherSettings;
pub use launcher_kinds::WebSearch;
pub use notifications::{Dnd, NotificationsSettings};
pub use osd::OsdSettings;
pub use schema::{schema, schemas};
pub use screenshot::{ScreenshotSettings, ShotThumbnail};
pub use scroll::{Momentum, NaturalScroll, ScrollSettings, ScrollbarVisibility};
pub use scrollbar::{ScrollbarSettings, TrackClick};
pub use session::SessionSettings;
pub use voice::{HoldToTalk, VoiceConsent, VoiceSettings, params as voice_params};
pub use wallpaper::WallpaperSettings;
pub use widgets::WidgetKind;

pub use ds_settings::{Count, Fraction, Mins, Ms, Percent, Px, Scalar, Secs};
