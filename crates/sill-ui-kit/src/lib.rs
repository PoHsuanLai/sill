//! What every surface shares: the [`root`] every surface document starts with, the [`surface`]
//! seam the daemon spawns them through, the [`links`] the daemon and the surfaces talk through,
//! the [`hooks`] that read services and settings, the active workspace's [`look`], the timed
//! [`machine`]s, [`menu`] popups, [`status`] glyphs, the [`notification_card`] and the
//! [`widgets`] more than one surface draws, the app icons, and (behind the `harness` feature)
//! the fake services and the harness that drive a surface in a test. The surfaces themselves
//! live in `sill-overlays`, `sill-notify-ui`, `sill-widgets-ui`, `sill-dock`,
//! `sill-launcher-ui` and `sill-bar`.
//!
//! Surfaces read shared state only through [`hooks`], which read `crate::services::Services` and
//! `settings::SettingsFeed` from the root context with `shell_host::use_watch`. Nothing
//! here passes a Dioxus signal through `Root::context` (a review rule), styles a
//! `.ds-*` class, or writes raw `button`/`input`/menu markup (coherence rules 1-2).

pub mod app_icons;
pub mod app_menu;
pub mod companion;
pub mod dock_rects;
#[cfg(feature = "harness")]
pub mod harness;
pub mod hooks;
pub mod links;
pub mod look;
pub mod menu;
pub mod notification_card;
pub mod panel;
pub mod root;
pub mod services;
pub mod settings;
pub mod shell_style;
pub mod status;
pub mod surface;
pub mod widgets;
