//! `$XDG_CONFIG_HOME/sill/settings.toml`: the shell's settings domains (design/22-SETTINGS.md
//! §3.4-3.8 and §3.11-3.14: bar, dock, launcher, scroll, scrollbar, menus, switcher, sound,
//! notifications, control center, spaces), their schema (§9, `#[derive(SettingsSchema)]` with
//! `file = "sill/settings.toml"`), lenient load and atomic save through `ds-settings`, a
//! directory watch, and the per-domain diff (§4.4).
//!
//! A crate of its own rather than a module of `sill-services`, because `ds-settings` brings
//! `dioxus` with it and `sill-services` stays UI-free (`scripts/check-boundary.sh`).

pub mod bar;
pub mod control_center;
pub mod diff;
pub mod dock;
mod dock_words;
pub mod feed;
pub mod file;
pub mod launcher;
pub mod menus;
pub mod notifications;
pub mod schema;
pub mod scroll;
pub mod scrollbar;
pub mod sound;
pub mod spaces;
pub mod switcher;
pub mod watch;

pub use bar::{BarGlyphSize, BarSettings};
pub use control_center::{ControlCenterModule, ControlCenterSettings, PanelMaterial};
pub use diff::{SettingsChange, apply};
pub use dock::{
    ActiveClick, AutoHide, Bounce, DockFloor, DockMenuOrder, DockModifierClicks, DockPosition,
    DockSettings, Indicators, LaunchAnim, Magnification, ProgressStyle, TrashTile,
};
pub use feed::SettingsFeed;
pub use file::{APP, FILE_NAME, ShellFile, load, save, settings_dir};
pub use launcher::LauncherSettings;
pub use menus::{MenusSettings, PickFeedback};
pub use notifications::{BannerEntry, BannerMaterial, BannerStyle, Dnd, NotificationsSettings};
pub use schema::{schema, schemas};
pub use scroll::{
    Momentum, MomentumModel, NaturalScroll, RubberBand, ScrollSettings, ScrollbarVisibility,
};
pub use scrollbar::{ScrollbarSettings, TrackClick};
pub use sound::{SoundSettings, UiSounds, VolumeFeedback};
pub use spaces::{
    DefaultCardAccent, DockLookSource, MailSpacePolicy, OverlayTint, SpaceLookLookup,
    SpacesSettings, WallpaperDrawer, WallpaperPolicy,
};
pub use switcher::SwitcherSettings;
pub use watch::{ShellWatch, watch};

pub use ds_settings::{Count, Fraction, Ms, Percent, Px, Scalar};
