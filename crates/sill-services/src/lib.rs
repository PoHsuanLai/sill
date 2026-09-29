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

pub use apps::{
    App, AppIcon, AppIndex, AppsCommand, AppsEvent, AppsService, DbusActivation,
    DesktopActionEntry, ExecArgs, ExecError, Listing, Terminal, expand_exec, match_app,
};
pub use auth::{Auth, AuthCommand, AuthPhase, AuthService};
pub use badges::{
    Badge, BadgeEvent, BadgeProp, Badges, BadgesCommand, BadgesService, parse_app_uri,
};
pub use battery::BatteryService;
pub use bluetooth::{
    Bluetooth, BluetoothCommand, BluetoothEvent, BluetoothService, BtAddress, BtBattery,
    BtConnection, BtDevice, BtKind, BtPairing, BtPower, device_kind, listed_devices,
};
pub use clipboard::{Clipboard, ClipboardCommand, ClipboardService};
pub use clock::{ClockCommand, ClockService, ClockText, format_clock, until_next_minute};
pub use control::Control;
pub use cosmic_wl::{
    Activity, CosmicWl, CosmicWlService, Pinning, Relative, Sizing, Toplevel, ToplevelEvent,
    Toplevels, Urgency, WindowState, WlCommand, Workspace, WorkspaceCap, WorkspaceCaps,
    WorkspaceEvent, WorkspaceGroup, Workspaces,
};
pub use displays::{
    BrightnessTarget, DisplayOutput, DisplayPolicy, DisplayScale, Displays, DisplaysCommand,
    DisplaysService, Following, FontRenderingMode, Gamut, HardwareVolume, IdentityKey, KeysTarget,
    LevelChange, NightWarmth, Ppi, ScaleMode, ScaleOverride, ScaleRounding,
};
pub use error::ServiceError;
pub use events::{Agenda, CalEvent, EventTime, EventsCommand, EventsService, SourceStatus};
pub use idle::{Idle, IdleCommand, IdleLinks, IdleService};
pub use kbd_backlight::{
    KbdBacklight, KbdBacklightService, KbdCommand, KbdEvent, KbdLevel, step_for,
};
pub use launch::{
    LaunchCommand, LaunchOutcome, LaunchRequest, LaunchService, LaunchedScope, Launches, ScopeUnit,
};
pub use locker::{
    Lock, LockCause, LockPhase, LockerCommand, LockerMode, LockerService, ShellLockEvent,
};
pub use magnify::{Layout, MagnifyParams, TileBox, magnify, rest_centers, sizes, zoom};
pub use network::wifi::{
    AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiCommand, WifiEvent,
    WifiService, listed_networks,
};
pub use network::{
    Activating, Connectivity, Link, Network, NetworkCommand, NetworkEvent, NetworkService,
};
pub use notifications::{
    Action, ActionKey, Activation, ActivationToken, AppGroup, AppKey, AppRef, BannerLife, Body,
    BodyRun, Chime, Clear, CloseReason, Expiry, Hints, History, HistoryCap, Href, IconRef,
    LiveBanner, NameHolder, NameOwner, NoteImage, NotesState, Notification, NotificationId,
    Notifications, NotificationsCommand, NotificationsEffect, NotificationsEvent,
    NotificationsPolicy, NotificationsService, NotifyUrgency, Pixels, Posting, Presentation,
    Presented, Quiet, Residency, RunLink, RunStyle, ServerMode, ServerStatus, Slant, SoundHint,
    Transience, Underline, UnixMs, Weight, assign_id, parse_body,
};
pub use now_playing::{
    Abilities, Art, ArtSource, Media, MediaCommand, MediaEvent, MediaPlayer, MediaVerb, Micros,
    NowPlayingService, Playback, PlayerId, Timeline, Track, art_cache_name, art_source,
};
pub use pins::{DockPins, PinAt, PinEvent, PinsService};
pub use polkit_agent::{Polkit, PolkitAgentService, PolkitCommand};
pub use power_profile::{
    PowerProfileService, PowerProfiles, Profile, ProfileCommand, ProfileDegraded, ProfileEvent,
};
pub use results::{
    CurrencyPolicy, FilesChoice, LauncherPolicy, Results, ResultsEvent, ResultsService,
    SearchCommand, WebPolicy,
};
pub use screenshots::{
    ScreenshotsService, Shot, ShotCommand, ShotId, ShotKind, ShotLeave, ShotPreview, Shots,
};
pub use service::{Handle, Service, ServiceIo};
pub use services::{Member, Services};
pub use session::{Offer, Session, SessionAction, SessionCommand, SessionEvent, SessionService};
pub use switcher::{AppShown, SwitcherApp, switcher_apps};
pub use tray::{
    Availability, Check, Pixmap, ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayIcon,
    TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind, TrayService, TrayStatus,
};
pub use volume::{Mute, Step, Volume, VolumeCommand, VolumeEvent, VolumeService, stepped};
pub use wallpaper::{
    WallpaperCommand, WallpaperImage, WallpaperService, WallpaperSource, Wallpapers,
    cache_file_name,
};
