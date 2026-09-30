//! What every surface shares, and the surfaces: the [`root`] every surface document starts with,
//! the [`links`] the daemon and the surfaces talk through, the [`hooks`] that read services and
//! settings, the active workspace's [`look`], the timed [`machine`]s, [`menu`] popups, [`status`]
//! glyphs, [`widgets`] both the notification center and the desktop draw, and the surfaces:
//! bar, wallpaper, dock, launcher, OSD, control center, power menu, notification banners and
//! notification center, the screenshot thumbnail, hot corners, the desktop widget layer, and the
//! own lock screen, polkit prompt and app switcher.
//!
//! Surfaces read shared state only through [`hooks`], which read `crate::services::Services` and
//! `settings::SettingsFeed` from the root context with `shell_host::use_watch`. Nothing
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
pub mod notification_card;
pub mod panel;
pub mod root;
pub mod services;
pub mod settings;
pub mod shell_style;
pub mod status;
pub mod surface;
pub mod surfaces;
pub mod widgets;

pub use surfaces::bar::Bar;
pub use surfaces::dock::Dock;
pub use surfaces::launcher::{Launcher, LauncherCatcher};
