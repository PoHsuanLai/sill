//! The shell's surfaces and hooks: bar, wallpaper, dock, launcher, OSD, control center, power
//! menu, notification banners and notification center, the screenshot thumbnail, hot corners,
//! the desktop widget layer, and the own lock screen, polkit prompt and app switcher (M11) (plan "Design: `<shell>` repo", `sill-surfaces`), the
//! [`widgets`] both the notification center and the desktop draw, plus the pure machines
//! design/10 and design/13 specify for them.
//!
//! Surfaces read shared state only through [`hooks`], which read `sill_services::Services` and
//! `sill_settings::SettingsFeed` from the root context with `shell_host::use_watch`. Nothing
//! here passes a Dioxus signal through `Root::context` (ORCHESTRATION review rule), styles a
//! `.ds-*` class, or writes raw `button`/`input`/menu markup (coherence rules 1-2).

pub mod app_icons;
pub mod clock;
pub mod hooks;
pub mod surfaces;
pub mod widgets;

pub use clock::FrameClock;
/// The rect measurer every root the daemon spawns is given (`Root::context(MEASURE)`): quire's
/// own, `ds_native::measure::MEASURE` (FINDINGS "Adoption of the bar gaps"). A popup's root
/// component calls `ds_native::measure::provide()` instead.
pub use ds_native::measure::MEASURE;

/// The pre-paint pass the daemon installs on its host (`HostConfig::prepaint`): quire's
/// device-pixel layout snap, a no-op at whole scales (quire FINDINGS "Pixel snapping").
pub fn prepaint() -> shell_host::Prepaint {
    shell_host::Prepaint::hook(ds_native::snap_to_device)
}

/// quire's caret read (`HostCaret`, Q299), for a root to carry beside [`FOCUS`] so a palette's
/// key claim sees where the caret is. The launcher's panel provides it itself
/// (`ds_native::focus::provide()`), so it does not wait on the daemon's roots.
pub use ds_native::focus::CARET;
/// quire's focus write every root the daemon spawns is given beside [`MEASURE`]
/// (`Root::context(FOCUS)`), so a field's or a menu's focus waits out a busy document instead
/// of printing a caught panic (FINDINGS "Adoption of the launcher gaps"). A popup's root
/// component calls `ds_native::focus::provide()` instead.
pub use ds_native::focus::FOCUS;
pub use surfaces::banners::Banners;
pub use surfaces::bar::Bar;
pub use surfaces::desktop_widgets::DesktopWidgets;
pub use surfaces::dock::Dock;
pub use surfaces::hot_corners::HotCorner;
pub use surfaces::launcher::{Launcher, LauncherCatcher};
pub use surfaces::lock::LockScreen;
pub use surfaces::notification_center::NotificationCenter;
pub use surfaces::osd::Osd;
pub use surfaces::polkit_prompt::PolkitPrompt;
pub use surfaces::power_menu::PowerMenu;
pub use surfaces::screenshot_thumb::ScreenshotThumb;
pub use surfaces::switcher::SwitcherPanel;
pub use surfaces::wallpaper::Wallpaper;
