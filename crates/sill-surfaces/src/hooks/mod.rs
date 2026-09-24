//! One hook per source (plan hooks table). Every body is `use_context` + `use_watch`: a hook
//! never owns a channel, spawns a service or reads a file.

mod use_appearance;
mod use_apps;
mod use_badges;
mod use_battery;
mod use_clock;
mod use_launches;
mod use_network;
mod use_pins;
mod use_results;
mod use_services;
mod use_settings;
mod use_space_look;
mod use_toplevels;
mod use_tray;
mod use_volume;
mod use_wallpapers;
mod use_workspaces;

pub use use_appearance::use_appearance;
pub use use_apps::use_apps;
pub use use_badges::use_launcher_entry;
pub use use_battery::use_battery;
pub use use_clock::use_clock;
pub use use_launches::use_launches;
pub use use_network::use_network;
pub use use_pins::use_pins;
pub use use_results::use_results;
pub use use_services::use_services;
pub use use_settings::use_settings;
pub use use_space_look::use_space_look;
pub use use_toplevels::use_toplevels;
pub use use_tray::use_tray;
pub use use_volume::use_volume;
pub use use_wallpapers::use_wallpapers;
pub use use_workspaces::use_workspaces;
