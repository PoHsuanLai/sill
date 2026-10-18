//! The notification surfaces: the banner column, the notification center and the screenshot
//! thumbnail. One directory per surface, each implementing `sill_ui_kit::surface::Surface`; the
//! cards they draw come from `sill_ui_kit::notification_card`.

pub mod banners;
mod keyboard;
pub mod notification_center;
pub mod screenshot_thumb;

pub use banners::Banners;
pub use notification_center::NotificationCenter;
pub use screenshot_thumb::ScreenshotThumb;
