//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, the lock screen's password check and the polkit agent, notifications,
//! calendar events, screenshots, volume, tray, apps, launching, dock pins, wallpaper, displays,
//! the windows and workspaces and the keycap keyboard daemon.
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

mod account_label;
pub mod accounts;
pub mod app_menu;
pub mod apps;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
mod call_error;
pub mod callers;
pub mod capture_pick;
pub mod clipboard;
pub mod clock;
pub mod companion;
pub mod control;
pub mod deadline;
mod debounced;
pub mod desktop_wl;
pub mod displays;
pub mod durable;
pub mod env;
pub mod error;
pub mod events;
pub mod first_run;
pub mod guest_asks;
pub mod idle;
pub mod input_source;
pub mod intents;
pub mod kbd_backlight;
pub mod keycap;
pub mod launch;
pub mod locker;
pub mod login_session;
mod machine;
pub mod model_offer;
pub mod network;
pub mod night_shift;
mod notices;
pub mod notifications;
pub mod now_playing;
mod owner;
pub mod pins;
pub mod polkit_agent;
pub mod power_profile;
mod restart;
pub mod screenshots;
pub mod seat;
mod serve;
pub mod service;
pub mod session;
mod stream;
pub mod supervise;
pub mod sync_hold;
pub mod tray;
pub mod voice;
pub mod volume;
pub mod wallpaper;
mod wayland_thread;
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
pub use desktop_wl::{WindowsLinks, WindowsService};
pub use displays::DisplaysService;
pub use error::ServiceError;
pub use events::EventsService;
pub use idle::{IdleLinks, IdleService};
pub use input_source::InputSourceService;
pub use intents::{IntentsLinks, IntentsService, ShellCall};
pub use kbd_backlight::KbdBacklightService;
pub use keycap::{KeycapLinks, KeycapService};
pub use launch::LaunchService;
pub use locker::LockerService;
pub use network::NetworkService;
pub use night_shift::NightShiftService;
pub use notifications::{Answers, NotificationsService};
pub use now_playing::NowPlayingService;
pub use pins::PinsService;
pub use polkit_agent::PolkitAgentService;
pub use power_profile::PowerProfileService;
pub use screenshots::ScreenshotsService;
pub use service::{Handle, Service, ServiceIo};
pub use session::{SessionLinks, SessionService};
pub use tray::TrayService;
pub use voice::{VoiceLinks, VoiceService};
pub use volume::VolumeService;
pub use wallpaper::WallpaperService;
pub use wifi::WifiService;
pub use work_areas::WorkAreaService;
pub use workspace_looks::WorkspaceLooksService;
