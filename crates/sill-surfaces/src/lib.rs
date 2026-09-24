//! The shell's surfaces and hooks: bar, wallpaper, dock and launcher (plan "Design: `<shell>`
//! repo", `sill-surfaces`), plus the pure machines design/10 and design/13 specify for them.
//!
//! Surfaces read shared state only through [`hooks`], which read `sill_services::Services` and
//! `sill_settings::SettingsFeed` from the root context with `shell_host::use_watch`. Nothing
//! here passes a Dioxus signal through `Root::context` (ORCHESTRATION review rule), styles a
//! `.ds-*` class, or writes raw `button`/`input`/menu markup (coherence rules 1-2).

pub mod clock;
pub mod hooks;
pub mod measure;
pub mod surfaces;

pub use clock::FrameClock;
pub use measure::MEASURE;
pub use surfaces::bar::Bar;
pub use surfaces::dock::Dock;
pub use surfaces::launcher::{Launcher, LauncherCatcher};
pub use surfaces::wallpaper::Wallpaper;
