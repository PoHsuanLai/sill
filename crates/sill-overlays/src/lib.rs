//! The overlay surfaces: the OSD, the power menu, the polkit prompt, the confirmation sheet, the voice consent sheet, the app switcher, the idle
//! dim, the hot corners, the wallpaper and the lock screen. One directory per surface, each
//! implementing `sill_ui_kit::surface::Surface` (the lock screen `LockSurface`).

pub mod confirm_sheet;
pub mod hot_corners;
pub mod idle_dim;
pub mod lock;
pub mod osd;
pub mod polkit_prompt;
pub mod power_menu;
pub mod switcher;
pub mod user_picture;
pub mod voice_consent;
pub mod wallpaper;

pub use confirm_sheet::ConfirmSheet;
pub use hot_corners::HotCorners;
pub use idle_dim::IdleDim;
pub use lock::Lock;
pub use osd::Osd;
pub use polkit_prompt::PolkitPrompt;
pub use power_menu::PowerMenu;
pub use switcher::Switcher;
pub use voice_consent::VoiceConsentSheet;
pub use wallpaper::Wallpaper;
