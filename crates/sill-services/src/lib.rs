//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, the lock screen's password check and the polkit agent, notifications,
//! calendar events, screenshots, volume, tray, apps, launching, dock pins, wallpaper, displays,
//! COSMIC Wayland state and the keycap keyboard daemon.
//!
//! Every service is one module with the same shape: a marker type implementing [`Service`],
//! started by [`service::spawn`] against [`env::Env`]'s seams into a [`Handle`] (a watch
//! receiver of its state and a [`Control`] for its commands), and a `backend` that carries its
//! effects. Its state, commands and the pure `step` that is the only place the state changes
//! live in `sill-model`, so every transition is table-tested without a bus or a socket. Shared
//! data travels on `tokio::sync::watch`, never as a Dioxus signal.
//!
//! This crate never reaches `dioxus` (`scripts/check-boundary.sh`): the hooks that read these
//! channels live in `sill-ui-kit::hooks`.

pub mod accounts;
pub mod app_menu;
pub mod apps;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod clock;
pub mod companion;
pub mod control;
pub mod cosmic_wl;
pub mod desktop_wl;
pub mod displays;
pub mod env;
pub mod error;
pub mod events;
mod file_touch;
pub mod guest_asks;
pub mod idle;
pub mod intents;
pub mod kbd_backlight;
pub mod keycap;
pub mod launch;
pub mod locker;
pub mod network;
pub mod notifications;
pub mod now_playing;
mod owner;
pub mod pins;
pub mod polkit_agent;
pub mod power_profile;
pub mod screenshots;
pub mod seat;
pub mod service;
pub mod session;
mod stream;
pub mod sync_hold;
pub mod tray;
pub mod voice;
pub mod volume;
pub mod wallpaper;
pub mod wifi;
pub mod work_areas;
pub mod workspace_looks;

pub use accounts::{AccountsLinks, AccountsService};
pub use app_menu::{AppMenuDeps, AppMenuService, MenuLinks, MenuNameOf};
pub use apps::AppsService;
pub use auth::AuthService;
pub use badges::BadgesService;
pub use battery::BatteryService;
pub use bluetooth::BluetoothService;
pub use clipboard::ClipboardService;
pub use clock::ClockService;
pub use companion::{CompanionLinks, CompanionService};
pub use control::Control;
pub use cosmic_wl::CosmicWlService;
pub use displays::DisplaysService;
pub use error::ServiceError;
pub use events::EventsService;
pub use idle::{IdleLinks, IdleService};
pub use intents::{IntentsLinks, IntentsService, ShellCall};
pub use kbd_backlight::KbdBacklightService;
pub use keycap::{KeycapLinks, KeycapService};
pub use launch::LaunchService;
pub use locker::LockerService;
pub use network::NetworkService;
pub use notifications::NotificationsService;
pub use now_playing::NowPlayingService;
pub use pins::PinsService;
pub use polkit_agent::PolkitAgentService;
pub use power_profile::PowerProfileService;
pub use screenshots::ScreenshotsService;
pub use service::{Handle, Service, ServiceIo};
pub use session::SessionService;
pub use tray::TrayService;
pub use voice::{VoiceLinks, VoiceService};
pub use volume::VolumeService;
pub use wallpaper::WallpaperService;
pub use wifi::WifiService;
pub use work_areas::WorkAreaService;
pub use workspace_looks::WorkspaceLooksService;
