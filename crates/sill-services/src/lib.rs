//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, the lock screen's password check and the polkit agent, notifications,
//! calendar events, screenshots, volume, tray, apps, launcher entries, dock pins, launcher search,
//! wallpaper, sounds, displays and COSMIC Wayland state.
//!
//! Every service is one module with the same shape: `spawn(rt) -> (watch::Receiver<T>,
//! Control<C>)` starts the effect, and a pure `reduce_*(T, Event) -> T` is the only place its
//! state changes, so every transition is table-tested without a bus or a socket. [`Services`]
//! bundles the receivers and controls into the one `Clone` value every surface gets through
//! `Root::context`. Shared data travels on `tokio::sync::watch`, never as a Dioxus signal.
//!
//! This crate never reaches `dioxus` (`scripts/check-boundary.sh`): the hooks that read these
//! channels live in `sill-surfaces::hooks`.

pub mod apps;
pub mod attempt;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod clock;
pub mod control;
pub mod cosmic_wl;
pub mod curve;
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
pub mod programs;
pub mod providers;
pub mod results;
pub mod screenshots;
pub mod service;
pub mod services;
pub mod session;
mod stream;
pub mod switcher;
pub mod tray;
pub mod units;
pub mod volume;
pub mod wallpaper;

pub use apps::{
    App, AppIcon, AppIndex, AppsCommand, AppsEvent, DbusActivation, DesktopActionEntry, ExecArgs,
    ExecError, Listing, Terminal, expand_exec, match_app, reduce_apps,
};
pub use attempt::{Attempt, AttemptStep, Attempts, OpStamp};
pub use auth::{Auth, AuthCommand, AuthPhase};
pub use badges::{
    Badge, BadgeEvent, BadgeProp, Badges, BadgesCommand, parse_app_uri, reduce_badges,
};
pub use battery::{Battery, BatteryCommand, BatteryEvent, BatteryService, Charge, Remaining};
pub use bluetooth::{
    Bluetooth, BluetoothCommand, BluetoothEvent, BluetoothService, BtAddress, BtBattery,
    BtConnection, BtDevice, BtKind, BtPairing, BtPower, device_kind, listed_devices,
};
pub use clipboard::{Clipboard, ClipboardCommand};
pub use clock::{ClockCommand, ClockService, ClockText, format_clock, until_next_minute};
pub use control::Control;
pub use cosmic_wl::{
    Activity, GroupKey, OutputName, Pinning, Relative, Sizing, Toplevel, ToplevelEvent, Toplevels,
    Urgency, WindowState, WlCommand, Workspace, WorkspaceCap, WorkspaceCaps, WorkspaceEvent,
    WorkspaceGroup, WorkspaceKey, Workspaces, reduce_toplevels, reduce_workspaces,
};
pub use curve::ease;
pub use displays::{
    BrightnessTarget, DisplayOutput, DisplayPolicy, DisplayScale, Displays, DisplaysCommand,
    Following, FontRenderingMode, Gamut, HardwareVolume, IdentityKey, KeysTarget, LevelChange,
    NightWarmth, Ppi, ScaleMode, ScaleOverride, ScaleRounding,
};
pub use error::ServiceError;
pub use events::{Agenda, CalEvent, EventTime, EventsCommand, SourceStatus};
pub use idle::{Idle, IdleCommand, IdleLinks};
pub use kbd_backlight::{
    KbdBacklight, KbdBacklightService, KbdCommand, KbdEvent, KbdLevel, step_for,
};
pub use launch::{
    LaunchCommand, LaunchOutcome, LaunchRequest, LaunchedScope, Launches, ScopeUnit,
    reduce_launches,
};
pub use locker::{Lock, LockCause, LockPhase, LockerCommand, LockerMode, ShellLockEvent};
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
    BodyRun, Chime, Clear, CloseReason, Expiry, Hints, History, HistoryCap, HistoryPlace, Href,
    IconRef, LiveBanner, NameHolder, NameOwner, NoteImage, NotesState, Notification,
    NotificationId, Notifications, NotificationsCommand, NotificationsEffect, NotificationsEvent,
    NotificationsPolicy, NotifyUrgency, Pixels, Posting, Presentation, Presented, Quiet, Residency,
    RunLink, RunStyle, ServerMode, ServerStatus, Setup, Slant, SoundHint, Transience, Underline,
    UnixMs, Weight, assign_id, parse_body, reduce_notifications,
};
pub use now_playing::{
    Abilities, Art, ArtSource, Media, MediaCommand, MediaEvent, MediaPlayer, MediaVerb, Micros,
    Playback, PlayerId, Timeline, Track, art_cache_name, art_source, reduce_media,
};
pub use pins::{DockPins, PinAt, PinEvent, reduce_pins};
pub use polkit_agent::{Polkit, PolkitCommand};
pub use power_profile::{
    PowerProfiles, Profile, ProfileCommand, ProfileDegraded, ProfileEvent, reduce_power_profiles,
};
pub use results::{
    CurrencyPolicy, FilesChoice, LauncherPolicy, Results, ResultsEvent, SearchCommand, WebPolicy,
    reduce_results,
};
pub use screenshots::{Shot, ShotCommand, ShotId, ShotKind, ShotLeave, ShotPreview, Shots};
pub use service::{Handle, Service, ServiceIo};
pub use services::Services;
pub use session::{Offer, Session, SessionAction, SessionCommand, SessionEvent, reduce_session};
pub use switcher::{AppShown, SwitcherApp, switcher_apps};
pub use tray::{
    Availability, Check, Pixmap, ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayIcon,
    TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind, TrayStatus, reduce_tray,
};
pub use units::{Cpx, Minutes, Percent, Permille, Presence, Seconds, Stamp, Visibility};
pub use volume::{Mute, Step, Volume, VolumeCommand, VolumeEvent, reduce_volume, stepped};
pub use wallpaper::{
    WallpaperCommand, WallpaperImage, WallpaperSource, Wallpapers, cache_file_name,
};
