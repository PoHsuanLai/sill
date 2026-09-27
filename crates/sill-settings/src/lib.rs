//! `$XDG_CONFIG_HOME/sill/settings.toml`: the shell's settings domains (design/22-SETTINGS.md
//! §3.4-3.8 and §3.11-3.14: bar, dock, launcher, scroll, scrollbar, menus, switcher, sound,
//! notifications, control center, spaces; `display`, `osd`, `power_menu` and `session`; and
//! M10's `widgets`, `calendar`, `screenshot` and `hot_corners`, which design/22 has no rows for
//! yet), their schema (§9, `#[derive(SettingsSchema)]` with
//! `file = "sill/settings.toml"`), lenient load and atomic save through `ds-settings`, a
//! directory watch, and the per-domain diff (§4.4).
//!
//! A crate of its own rather than a module of `sill-services`, because `ds-settings` brings
//! `dioxus` with it and `sill-services` stays UI-free (`scripts/check-boundary.sh`).

pub mod bar;
pub mod calendar;
pub mod control_center;
pub mod diff;
pub mod display;
pub mod dock;
mod dock_words;
pub mod feed;
pub mod file;
pub mod focus;
pub mod focus_link;
pub mod hot_corners;
pub mod idle;
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
pub mod session;
pub mod sound;
pub mod spaces;
pub mod switcher;
pub mod watch;
pub mod widgets;
pub mod window;

pub use bar::{BarGlyphSize, BarSettings};
pub use calendar::{CalendarSettings, FirstWeekday, WeekNumbers};
pub use control_center::{ControlCenterModule, ControlCenterSettings, InMenuBar, PanelMaterial};
pub use diff::{SettingsChange, apply};
pub use display::{
    BrightnessKeysTarget, ColorManagement, DisplayScaleMode, DisplaySettings, FontRenderingSetting,
    HardwareVolumeMode, NightWarmthMode, ScaleRounding,
};
pub use dock::{
    ActiveClick, AutoHide, Bounce, DockFloor, DockMenuOrder, DockModifierClicks, DockPosition,
    DockSettings, Indicators, LaunchAnim, Magnification, ProgressStyle, TrashTile,
};
pub use feed::SettingsFeed;
pub use file::{APP, FILE_NAME, ShellFile, load, save, settings_dir};
pub use focus::{FocusMode, focus_mode, with_focus_mode};
pub use focus_link::{Delivery, FocusLink, spawn_focus};
pub use hot_corners::{CornerAction, HotCornersSettings};
pub use idle::{IdleLockAt, IdleSettings, IdleTimesFrom};
pub use launcher::LauncherSettings;
pub use launcher_kinds::{ClipboardHistory, CurrencyRates, EmojiSkinTone, FilesBackend, WebSearch};
pub use menus::{MenusSettings, PickFeedback};
pub use notifications::{
    BannerEntry, BannerMaterial, BannerPosition, BannerStyle, Dnd, NotificationServer,
    NotificationSwipe, NotificationsSettings,
};
pub use osd::{OsdEnabled, OsdPosition, OsdSettings};
pub use power_menu::{PowerMenuDefault, PowerMenuSettings};
pub use schema::{schema, schemas};
pub use screenshot::{ScreenshotSettings, ShotThumbnail};
pub use scroll::{
    Momentum, MomentumModel, NaturalScroll, RubberBand, ScrollSettings, ScrollbarVisibility,
};
pub use scrollbar::{ScrollbarSettings, TrackClick};
pub use session::{LockClock, SessionSettings};
pub use sound::{SoundSettings, UiSounds, VolumeFeedback};
pub use spaces::{
    DefaultCardAccent, DockLookSource, MailSpacePolicy, OverlayTint, SpaceLookLookup,
    SpacesSettings, WallpaperDrawer, WallpaperPolicy,
};
pub use switcher::SwitcherSettings;
pub use watch::{ShellWatch, watch};
pub use widgets::{DesktopWidgets, WidgetKind, WidgetsSettings};
pub use window::WindowSettings;

pub use ds_settings::{Count, Fraction, Mins, Ms, Percent, Px, Scalar, Secs};
