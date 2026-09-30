//! The hooks every surface reads its sources through. [`use_service`] (and
//! [`use_service_part`], [`use_control`]) reach any service by its type; the rest read the
//! settings, the appearance and the focus mode. A hook never owns a channel, spawns a service or
//! reads a file.

mod use_appearance;
mod use_focus_mode;
mod use_service;
mod use_services;
mod use_settings;
mod use_user_style;

pub use use_appearance::{PinnedAppearance, use_appearance};
pub use use_focus_mode::use_focus_mode;
pub use use_service::{use_control, use_service, use_service_part};
pub use use_services::use_services;
pub use use_settings::{use_section, use_settings};
pub use use_user_style::use_user_style;
