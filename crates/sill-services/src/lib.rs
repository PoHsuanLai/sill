//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, volume, tray, apps, launcher entries, dock pins, launcher search,
//! wallpaper, sounds, displays and COSMIC Wayland state (plan
//! "Design: `<shell>` repo", `sill-services`).
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
pub mod badges;
pub mod battery;
pub mod bluetooth;
pub mod clock;
pub mod control;
pub mod cosmic_wl;
pub mod curve;
pub mod displays;
pub mod error;
pub mod launch;
pub mod magnify;
pub mod network;
pub mod now_playing;
pub mod pins;
pub mod power_profile;
pub mod providers;
pub mod results;
pub mod services;
pub mod session;
pub mod sound;
pub mod tray;
pub mod units;
pub mod volume;
pub mod wallpaper;

pub use apps::{
    App, AppIcon, AppIndex, AppsCommand, AppsEvent, DbusActivation, DesktopActionEntry, ExecArgs,
    ExecError, Listing, Terminal, expand_exec, match_app, reduce_apps,
};
pub use badges::{
    Badge, BadgeEvent, BadgeProp, Badges, BadgesCommand, parse_app_uri, reduce_badges,
};
pub use battery::{Battery, BatteryCommand, BatteryEvent, Charge, Remaining, reduce_battery};
pub use bluetooth::{
    Bluetooth, BluetoothCommand, BluetoothEvent, BtAddress, BtBattery, BtConnection, BtDevice,
    BtKind, BtPairing, BtPower, device_kind, listed_devices, reduce_bluetooth,
};
pub use clock::{ClockCommand, ClockText, format_clock, until_next_minute};
pub use control::Control;
pub use cosmic_wl::{
    Activity, GroupKey, OutputName, Pinning, Relative, Sizing, Toplevel, ToplevelEvent, Toplevels,
    Urgency, WindowState, WlCommand, Workspace, WorkspaceCap, WorkspaceCaps, WorkspaceEvent,
    WorkspaceGroup, WorkspaceKey, Workspaces, reduce_toplevels, reduce_workspaces,
};
pub use curve::ease;
pub use displays::{
    BrightnessTarget, DisplayOutput, DisplayPolicy, DisplayScale, Displays, DisplaysCommand,
    FontRenderingMode, Gamut, HardwareVolume, IdentityKey, KeysTarget, LevelChange, NightWarmth,
    Ppi, ScaleMode, ScaleOverride, ScaleRounding,
};
pub use error::ServiceError;
pub use launch::{
    LaunchCommand, LaunchOutcome, LaunchRequest, LaunchedScope, Launches, ScopeUnit,
    reduce_launches,
};
pub use magnify::{Layout, MagnifyParams, TileBox, magnify, rest_centers, sizes, zoom};
pub use network::wifi::{
    AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiCommand, WifiEvent,
    listed_networks, reduce_wifi,
};
pub use network::{Connectivity, Link, Network, NetworkCommand, NetworkEvent, reduce_network};
pub use now_playing::{
    Abilities, Art, ArtSource, Media, MediaCommand, MediaEvent, MediaPlayer, MediaVerb, Micros,
    Playback, PlayerId, Timeline, Track, art_cache_name, art_source, reduce_media,
};
pub use pins::{DockPins, PinAt, PinEvent, reduce_pins};
pub use power_profile::{
    PowerProfiles, Profile, ProfileBus, ProfileCommand, ProfileDegraded, ProfileEvent,
    reduce_power_profiles,
};
pub use results::{Results, ResultsEvent, SearchCommand, reduce_results};
pub use services::Services;
pub use session::{Offer, Session, SessionAction, SessionCommand, SessionEvent, reduce_session};
pub use sound::{SoundCommand, SoundName};
pub use tray::{
    Availability, Check, Pixmap, ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayIcon,
    TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind, TrayStatus, reduce_tray,
};
pub use units::{Cpx, Minutes, Percent, Permille, Presence, Seconds, Stamp, Visibility};
pub use volume::{Mute, Step, Volume, VolumeCommand, VolumeEvent, reduce_volume, stepped};
pub use wallpaper::{
    WallpaperCommand, WallpaperImage, WallpaperSource, Wallpapers, cache_file_name,
};
