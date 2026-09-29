//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, the lock screen's password check and the polkit agent, notifications,
//! calendar events, screenshots, volume, tray, apps, launcher entries, dock pins, launcher search,
//! wallpaper, sounds, displays and COSMIC Wayland state.
//!
//! Every service is one module with the same shape: a marker type implementing [`Service`],
//! started by [`service::spawn`] against [`env::Env`]'s seams into a [`Handle`] (a watch
//! receiver of its state and a [`Control`] for its commands), and a pure `step` beside its state
//! that is the only place the state changes, so every transition is table-tested without a bus
//! or a socket. [`Services`] holds every running service's handle, the one `Clone` value every
//! surface gets through `Root::context`. Shared data travels on `tokio::sync::watch`, never as a
//! Dioxus signal.
//!
//! This crate never reaches `dioxus` (`scripts/check-boundary.sh`): the hooks that read these
//! channels live in `sill-surfaces::hooks`.

pub mod apps;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod clock;
pub mod control;
pub mod cosmic_wl;
pub mod displays;
pub mod env;
pub mod error;
pub mod events;
pub mod idle;
pub mod kbd_backlight;
pub mod launch;
pub mod locker;
pub mod magnify;
pub mod network;
pub mod notifications;
pub mod now_playing;
pub mod pins;
pub mod polkit_agent;
pub mod power_profile;
pub mod preview;
pub mod providers;
pub mod results;
pub mod screenshots;
pub mod service;
pub mod services;
pub mod session;
mod stream;
pub mod switcher;
pub mod tray;
pub mod volume;
pub mod wallpaper;
pub mod wifi;

pub use apps::AppsService;
pub use auth::AuthService;
pub use badges::BadgesService;
pub use battery::BatteryService;
pub use bluetooth::BluetoothService;
pub use clipboard::{Clipboard, ClipboardCommand, ClipboardService};
pub use clock::ClockService;
pub use control::Control;
pub use cosmic_wl::CosmicWlService;
pub use displays::DisplaysService;
pub use error::ServiceError;
pub use events::{Agenda, CalEvent, EventTime, EventsCommand, EventsService, SourceStatus};
pub use idle::{IdleLinks, IdleService};
pub use kbd_backlight::KbdBacklightService;
pub use launch::{
    LaunchCommand, LaunchOutcome, LaunchRequest, LaunchService, LaunchedScope, Launches, ScopeUnit,
};
pub use locker::LockerService;
pub use magnify::{Layout, MagnifyParams, TileBox, magnify, rest_centers, sizes, zoom};
pub use network::NetworkService;
pub use notifications::{
    Action, ActionKey, Activation, ActivationToken, AppGroup, AppKey, AppRef, BannerLife, Body,
    BodyRun, Chime, Clear, CloseReason, Expiry, Hints, History, HistoryCap, Href, IconRef,
    LiveBanner, NameHolder, NameOwner, NoteImage, NotesState, Notification, NotificationId,
    Notifications, NotificationsCommand, NotificationsEffect, NotificationsEvent,
    NotificationsPolicy, NotificationsService, NotifyUrgency, Pixels, Posting, Presentation,
    Presented, Quiet, Residency, RunLink, RunStyle, ServerMode, ServerStatus, Slant, SoundHint,
    Transience, Underline, UnixMs, Weight, assign_id, parse_body,
};
pub use now_playing::NowPlayingService;
pub use pins::PinsService;
pub use polkit_agent::PolkitAgentService;
pub use power_profile::PowerProfileService;
pub use results::{
    CurrencyPolicy, FilesChoice, LauncherPolicy, Results, ResultsEvent, ResultsService,
    SearchCommand, WebPolicy,
};
pub use screenshots::{
    ScreenshotsService, Shot, ShotCommand, ShotId, ShotKind, ShotLeave, ShotPreview, Shots,
};
pub use service::{Handle, Service, ServiceIo};
pub use services::{Member, Services};
pub use session::SessionService;
pub use switcher::{AppShown, SwitcherApp, switcher_apps};
pub use tray::TrayService;
pub use volume::VolumeService;
pub use wallpaper::WallpaperService;
pub use wifi::WifiService;
