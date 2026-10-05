//! `$XDG_CONFIG_HOME/sill/settings.toml`: the shell's settings domains (design/22-SETTINGS.md
//! §3.4-3.8 and §3.11-3.14: bar, dock, launcher, scroll, scrollbar, menus, switcher, sound,
//! notifications, control center, spaces; `display`, `osd`, `power_menu` and `session`; and
//! `widgets`, `calendar`, `screenshot` and `hot_corners`; §3.24-3.25: `idle` and `keyboard`; §3.30: `companion`), their schema (§9,
//! `#[derive(SettingsSchema)]` with `file = "sill/settings.toml"`), and [`Section`], each
//! domain as a reader finds it in a [`ShellFile`].
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
pub mod menus;
pub mod notifications;
pub mod osd;
pub mod power_menu;
pub mod schema;
pub mod screenshot;
pub mod scroll;
pub mod scrollbar;
mod section;
pub mod session;
pub mod spaces;
pub mod switcher;
pub mod widgets;
pub mod window;

pub use bar::{BarGlyphSize, BarSettings};
pub use calendar::{CalendarSettings, FirstWeekday, WeekNumbers};
pub use companion::{
    CompanionSettings, InField, NotifyWhen, SummonKey, params as companion_params,
};
pub use control_center::{ControlCenterModule, ControlCenterSettings, InMenuBar, PanelMaterial};
pub use display::{
    BrightnessKeysTarget, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
    HardwareVolumeMode, NightWarmthMode, ScaleRounding,
};
pub use dock::{
    ActiveClick, AutoHide, Bounce, DockFloor, DockMenuOrder, DockModifierClicks, DockSettings,
    Indicators, LaunchAnim, Magnification, TrashTile,
};
pub use file::{APP, ShellFile};
pub use focus::{FocusMode, focus_mode, with_focus_mode};
pub use hot_corners::{CornerAction, HotCornerSettings};
pub use idle::{IdleLockAt, IdleSettings, IdleTimesFrom};
pub use keyboard::{KeyboardOverride, KeyboardSettings, RejectedRow, overrides};
pub use launcher::LauncherSettings;
pub use launcher_kinds::{ClipboardHistory, CurrencyRates, EmojiSkinTone, FilesBackend, WebSearch};
pub use menus::MenusSettings;
pub use notifications::{
    BannerMaterial, BannerPosition, BannerStyle, Dnd, NotificationServer, NotificationSwipe,
    NotificationsSettings,
};
pub use osd::{OsdEnabled, OsdPosition, OsdSettings};
pub use power_menu::{PowerMenuDefault, PowerMenuSettings};
pub use schema::{schema, schemas};
pub use screenshot::{ScreenshotSettings, ShotThumbnail};
pub use scroll::{
    Momentum, MomentumModel, NaturalScroll, RubberBand, ScrollSettings, ScrollbarVisibility,
};
pub use scrollbar::{ScrollbarSettings, TrackClick};
pub use section::Section;
pub use session::{LockClock, SessionSettings};
pub use spaces::{DefaultCardAccent, SpacesSettings, WallpaperDrawer};
pub use switcher::SwitcherSettings;
pub use widgets::{DesktopWidgets, WidgetKind, WidgetsSettings};
pub use window::WindowSettings;

pub use ds_settings::{Count, Fraction, Mins, Ms, Percent, Px, Scalar, Secs};
