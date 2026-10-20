//! The hooks every surface reads its sources through. [`use_service`] (and
//! [`use_service_part`], [`use_control`]) reach any service by its type; the rest read the
//! settings, the appearance and the focus mode, and [`use_follow_visibility`] maps a surface by a
//! reading. A hook never owns a channel, spawns a service or
//! reads a file.

mod use_appearance;
mod use_banner_output;
mod use_entrance;
mod use_focus_mode;
mod use_follow_visibility;
mod use_hours;
mod use_service;
mod use_services;
mod use_settings;
mod use_sheet_output;
mod use_user_style;

pub use use_appearance::{PinnedAppearance, SwitchableAppearance, use_appearance};
pub use use_banner_output::use_banner_output;
pub use use_entrance::{Entrance, use_entrance};
pub use use_focus_mode::use_focus_mode;
pub use use_follow_visibility::use_follow_visibility;
pub use use_hours::{PinnedLocale, use_hours};
pub use use_service::{use_control, use_service, use_service_part};
pub use use_services::use_services;
pub use use_settings::{use_section, use_settings};
pub use use_sheet_output::use_sheet_output;
pub use use_user_style::use_user_style;
