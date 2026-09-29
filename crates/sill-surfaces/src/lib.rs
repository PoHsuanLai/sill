//! The shell's surfaces and hooks: bar, wallpaper, dock, launcher, OSD, control center, power
//! menu, notification banners and notification center, the screenshot thumbnail, hot corners,
//! the desktop widget layer, and the own lock screen, polkit prompt and app switcher, the
//! [`widgets`] both the notification center and the desktop draw, plus the pure machines
//! design/10 and design/13 specify for them.
//!
//! Surfaces read shared state only through [`hooks`], which read `crate::services::Services` and
//! `sill_settings::SettingsFeed` from the root context with `shell_host::use_watch`. Nothing
//! here passes a Dioxus signal through `Root::context` (a review rule), styles a
//! `.ds-*` class, or writes raw `button`/`input`/menu markup (coherence rules 1-2).

pub mod app_icons;
pub mod clock;
#[cfg(feature = "harness")]
pub mod harness;
pub mod hooks;
pub mod links;
pub mod look;
pub mod machine;
pub mod menu;
pub mod root;
pub mod services;
pub mod status;
pub mod surfaces;
pub mod widgets;

pub use surfaces::banners::Banners;
pub use surfaces::bar::Bar;
pub use surfaces::desktop_widgets::DesktopWidgets;
pub use surfaces::dock::Dock;
pub use surfaces::edit_widgets::EditWidgets;
pub use surfaces::hot_corners::HotCorner;
pub use surfaces::idle_dim::IdleDimOverlay;
pub use surfaces::launcher::{Launcher, LauncherCatcher};
pub use surfaces::lock::LockScreen;
pub use surfaces::notification_center::NotificationCenter;
pub use surfaces::osd::Osd;
pub use surfaces::polkit_prompt::PolkitPrompt;
pub use surfaces::power_menu::PowerMenu;
pub use surfaces::screenshot_thumb::ScreenshotThumb;
pub use surfaces::switcher::SwitcherPanel;
pub use surfaces::wallpaper::Wallpaper;
