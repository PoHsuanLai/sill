//! The overlay surfaces: the OSD, the power menu, the polkit prompt, the confirmation sheet, the voice consent sheet, the app switcher, the idle
//! dim, the hot corners, the wallpaper, the window menu, Mission Control's Spaces bar, the Split View picker, the colour picker's eyedropper and the lock screen. One directory per surface, each
//! implementing `sill_ui_kit::surface::Surface` (the lock screen `LockSurface`).

pub mod accounts_sheet;
pub mod capture_sheet;
pub mod confirm_sheet;
pub mod eyedropper;
pub mod hot_corners;
pub mod idle_dim;
pub mod lock;
pub mod osd;
pub mod polkit_prompt;
pub mod power_menu;
pub mod spaces_bar;
pub mod split_picker;
pub mod switcher;
pub mod user_picture;
pub mod voice_consent;
pub mod wallpaper;
pub mod window_menu;

pub use accounts_sheet::AccountsSheet;
pub use capture_sheet::CaptureSheet;
pub use confirm_sheet::ConfirmSheet;
pub use eyedropper::ColorSampler;
pub use hot_corners::HotCorners;
pub use idle_dim::IdleDim;
pub use lock::Lock;
pub use osd::Osd;
pub use polkit_prompt::PolkitPrompt;
pub use power_menu::PowerMenu;
pub use spaces_bar::SpacesBar;
pub use split_picker::SplitPicker;
pub use switcher::Switcher;
pub use voice_consent::VoiceConsentSheet;
pub use wallpaper::Wallpaper;
pub use window_menu::WindowMenu;
