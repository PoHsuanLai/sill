//! Fake services for the bar's tests and `examples/bar_demo.rs`: every receiver is a `watch`
//! channel the caller holds the sender of, so a test feeds the frozen channels exactly as the
//! services will once they land.

#![allow(dead_code)] // Each test binary uses its own subset.

use shell_host::{OutputId, OutputInfo, Rect, Scale};
use sill_launcher::WindowKey;
use sill_model::apps::AppIndex;
use sill_model::badges::Badges;
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{
    Bluetooth, BluetoothCommand, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing,
    BtPower,
};
use sill_model::clock::ClockText;
use sill_model::cosmic_wl::{
    Activity, CosmicWl, Pinning, Sizing, Toplevel, Toplevels, WindowState, Workspace,
    WorkspaceCaps, WorkspaceGroup, Workspaces,
};
use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};
use sill_model::launch::Launches;
use sill_model::network::{Connectivity, Link, Network};
use sill_model::now_playing::{Media, MediaCommand};
use sill_model::pins::DockPins;
use sill_model::power_profile::{PowerProfiles, Profile, ProfileCommand, ProfileDegraded};
use sill_model::results::Results;
use sill_model::tray::Tray;
use sill_model::units::{Percent, Presence, Seconds, Urgency, Visibility};
use sill_model::volume::{Mute, Volume};
use sill_model::wallpaper::Wallpapers;
use sill_model::wifi::{AccessPoint, Association, Radio, Security, Ssid, Wifi, WifiCommand};
use sill_services::{Handle, Service};
use sill_surfaces::services::Services;
use tokio::sync::{mpsc, watch};

/// The senders behind a fake [`Services`], and the command queues the bar writes to.
#[derive(Debug)]
pub struct Feeds {
    pub clock: watch::Sender<ClockText>,
    pub battery: watch::Sender<Battery>,
    pub network: watch::Sender<Network>,
    pub volume: watch::Sender<Volume>,
    pub tray: watch::Sender<Tray>,
    pub cosmic_wl: watch::Sender<CosmicWl>,
    pub wifi: watch::Sender<Wifi>,
    pub bluetooth: watch::Sender<Bluetooth>,
    pub power_profiles: watch::Sender<PowerProfiles>,
    pub media: watch::Sender<Media>,
    /// The keyboard backlight: absent until a test feeds one (`kbd_backlight()`).
    pub kbd_backlight: watch::Sender<sill_model::kbd_backlight::KbdBacklight>,
    pub kbd_commands: mpsc::UnboundedReceiver<sill_model::kbd_backlight::KbdCommand>,
    /// The live banners and the history (both empty until a test feeds them).
    pub notifications: watch::Sender<sill_model::notifications::NotesState>,
    pub wl: mpsc::UnboundedReceiver<sill_model::cosmic_wl::WlCommand>,
    pub volume_commands: mpsc::UnboundedReceiver<sill_model::volume::VolumeCommand>,
    pub launch: mpsc::UnboundedReceiver<sill_model::launch::LaunchCommand>,
    pub wifi_commands: mpsc::UnboundedReceiver<WifiCommand>,
    pub bluetooth_commands: mpsc::UnboundedReceiver<BluetoothCommand>,
    pub profile_commands: mpsc::UnboundedReceiver<ProfileCommand>,
    pub media_commands: mpsc::UnboundedReceiver<MediaCommand>,
    pub notification_commands:
        mpsc::UnboundedReceiver<sill_model::notifications::NotificationsCommand>,
    /// The calendar widgets' agenda, the thumbnail's shots, and what the thumbnail sends.
    pub agenda: watch::Sender<sill_model::events::Agenda>,
    pub shots: watch::Sender<sill_model::screenshots::Shots>,
    pub shot_commands: mpsc::UnboundedReceiver<sill_model::screenshots::ShotCommand>,
    /// sill's own idle service (passive, `Off(NoPolicy)`, until a test feeds it).
    pub idle: watch::Sender<sill_model::idle::Idle>,
    /// The displays (none until a test feeds some), and what the Display module sends.
    pub displays: watch::Sender<sill_model::displays::Displays>,
    pub displays_commands: mpsc::UnboundedReceiver<sill_model::displays::DisplaysCommand>,
    /// Receivers and command queues nobody reads here, kept so sends do not fail.
    keep: Vec<Box<dyn std::any::Any>>,
}

/// A 1920 x 1080 output named `DP-1`.
pub fn output() -> OutputInfo {
    OutputInfo {
        id: OutputId(1),
        name: Some("DP-1".to_owned()),
        description: None,
        logical: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: Scale::ONE,
        refresh_mhz: 60_000,
    }
}

/// Three workspaces on `DP-1`, the second active.
pub fn workspaces() -> Workspaces {
    workspaces_on("DP-1")
}

/// Three workspaces on `output`, the second active.
pub fn workspaces_on(output: &str) -> Workspaces {
    let workspace = |key: u64, activity| Workspace {
        key: WorkspaceKey(key),
        name: String::new(),
        stable_id: String::new(),
        activity,
        urgency: Urgency::Calm,
        visibility: Visibility::Shown,
        coordinates: vec![u32::try_from(key).unwrap_or(u32::MAX)],
        capabilities: WorkspaceCaps::default(),
        pinning: Pinning::Unpinned,
    };
    Workspaces {
        groups: vec![WorkspaceGroup {
            key: GroupKey(1),
            outputs: vec![OutputName(output.to_owned())],
            workspaces: vec![
                workspace(1, Activity::Inactive),
                workspace(2, Activity::Active),
                workspace(3, Activity::Inactive),
            ],
        }],
    }
}

/// Two terminal windows on `DP-1`.
pub fn toplevels() -> Toplevels {
    toplevels_on("DP-1")
}

/// Two terminal windows on `output`.
pub fn toplevels_on(output: &str) -> Toplevels {
    let window = |key: u64, title: &str, activity| Toplevel {
        key: WindowKey(key),
        app_id: "foot".to_owned(),
        title: title.to_owned(),
        activity,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: vec![WorkspaceKey(2)],
        outputs: vec![OutputName(output.to_owned())],
    };
    Toplevels {
        windows: vec![
            window(10, "~/sill", Activity::Active),
            window(11, "cargo test", Activity::Inactive),
        ],
    }
}

/// The fixed values every test and the demo start from.
pub fn clock() -> ClockText {
    ClockText {
        text: "Thu 24 Sep 14:05".to_owned(),
    }
}

pub fn battery() -> Battery {
    Battery {
        presence: Presence::Present,
        level: Percent(64),
        charge: Charge::Discharging,
        time_to_empty: Seconds(7500),
        time_to_full: Seconds(0),
    }
}

pub fn network() -> Network {
    Network {
        link: Link::Wifi {
            ssid: "home".to_owned(),
            strength: Percent(72),
        },
        connectivity: Connectivity::Full,
        join: Default::default(),
    }
}

pub fn volume() -> Volume {
    Volume {
        presence: Presence::Present,
        level: Percent(40),
        mute: Mute::Audible,
        sink: "Built-in Audio Analog Stereo".to_owned(),
    }
}

/// Wi-Fi on, joined to `home`, two more networks in range.
pub fn wifi() -> Wifi {
    let ap = |ssid: &str, strength, security, association| AccessPoint {
        ssid: Ssid(ssid.to_owned()),
        strength: Percent(strength),
        security,
        association,
    };
    Wifi {
        device: Presence::Present,
        radio: Radio::On,
        access_points: vec![
            ap("home", 72, Security::Secured, Association::Connected),
            ap("office", 48, Security::Secured, Association::Idle),
            ap("cafe", 30, Security::Open, Association::Idle),
        ],
        saved: vec![Ssid("home".to_owned()), Ssid("office".to_owned())],
    }
}

/// Bluetooth on, headphones connected, a keyboard paired.
pub fn bluetooth() -> Bluetooth {
    let device = |address: &str, name: &str, kind, connection, battery| BtDevice {
        address: BtAddress(address.to_owned()),
        name: name.to_owned(),
        kind,
        connection,
        pairing: BtPairing::Paired,
        battery,
    };
    Bluetooth {
        presence: Presence::Present,
        power: BtPower::On,
        devices: vec![
            device(
                "AA:01",
                "Buds",
                BtKind::Headphones,
                BtConnection::Connected,
                BtBattery::Level(Percent(80)),
            ),
            device(
                "AA:02",
                "Keychron K5 Max",
                BtKind::Keyboard,
                BtConnection::Disconnected,
                BtBattery::Unknown,
            ),
        ],
        connect: Default::default(),
    }
}

/// A keyboard backlight with three steps, on the second.
pub fn kbd_backlight() -> sill_model::kbd_backlight::KbdBacklight {
    sill_model::kbd_backlight::KbdBacklight {
        presence: Presence::Present,
        level: sill_model::kbd_backlight::KbdLevel { value: 2, max: 3 },
    }
}

/// power-profiles-daemon on Balanced, all three offered.
pub fn power_profiles() -> PowerProfiles {
    PowerProfiles {
        presence: Presence::Present,
        active: Profile::Balanced,
        offered: Profile::ALL.to_vec(),
        degraded: ProfileDegraded::No,
    }
}

/// One player, playing "So What" (no art file), every button live.
pub fn media() -> Media {
    use sill_model::now_playing::{Abilities, MediaPlayer, Playback, PlayerId, Timeline, Track};
    use sill_model::units::Availability;
    use sill_model::units::Stamp;
    Media {
        players: vec![MediaPlayer {
            id: PlayerId("org.mpris.MediaPlayer2.fake".to_owned()),
            identity: "Fake Player".to_owned(),
            playback: Playback::Playing,
            changed: Stamp(1),
            track: Track {
                title: "So What".to_owned(),
                artists: vec!["Miles Davis".to_owned()],
                album: "Kind of Blue".to_owned(),
                ..Track::default()
            },
            timeline: Timeline::Unknown,
            abilities: Abilities {
                play_pause: Availability::Enabled,
                next: Availability::Enabled,
                previous: Availability::Enabled,
            },
        }],
    }
}

/// A handle at `state` whose sender and command queue are kept in `keep`.
fn fixed<S: Service>(state: S::State, keep: &mut Vec<Box<dyn std::any::Any>>) -> Handle<S> {
    let (handle, io) = Handle::channel(state);
    keep.push(Box::new(io));
    handle
}

/// Fake services at the fixed values above, and their senders.
pub fn services() -> (Services, Feeds) {
    let mut keep: Vec<Box<dyn std::any::Any>> = Vec::new();
    let (clock, clock_io) = Handle::channel(clock());
    let (battery, battery_io) = Handle::channel(battery());
    let (network, network_io) = Handle::channel(network());
    let (volume, volume_io) = Handle::channel(volume());
    let (tray, tray_io) = Handle::channel(Tray::default());
    let (cosmic_wl, cosmic_wl_io) = Handle::channel(CosmicWl {
        workspaces: workspaces(),
        toplevels: toplevels(),
    });
    let (wifi, wifi_io) = Handle::channel(wifi());
    let (bluetooth, bluetooth_io) = Handle::channel(bluetooth());
    let (power_profile, profile_io) = Handle::channel(power_profiles());
    // No player until a test or the demo feeds one (`media()`).
    let (now_playing, media_io) = Handle::channel(Media::default());
    let (launch, launch_io) = Handle::channel(Launches::default());
    let (kbd_backlight, kbd_io) =
        Handle::channel(sill_model::kbd_backlight::KbdBacklight::default());
    let (notifications, notifications_io) =
        Handle::channel(sill_model::notifications::NotesState::default());
    // A feed for the calendar widgets and one for the screenshot thumbnail.
    let (events, agenda_io) = Handle::channel(sill_model::events::Agenda::default());
    let (screenshots, shots_io) = Handle::channel(sill_model::screenshots::Shots::default());
    let (idle, idle_io) = Handle::channel(sill_model::idle::Idle::default());
    let (displays, displays_io) = Handle::channel(sill_model::displays::Displays::default());
    keep.push(Box::new(clock_io.commands));
    keep.push(Box::new(battery_io.commands));
    keep.push(Box::new(network_io.commands));
    keep.push(Box::new(tray_io.commands));
    keep.push(Box::new(idle_io.commands));
    let services = Services {
        clock,
        battery,
        network,
        volume,
        tray,
        cosmic_wl,
        apps: fixed(AppIndex::default(), &mut keep),
        badges: fixed(Badges::default(), &mut keep),
        pins: fixed(DockPins::default(), &mut keep),
        launch,
        results: fixed(Results::default(), &mut keep),
        wallpaper: fixed(Wallpapers::default(), &mut keep),
        displays,
        wifi,
        bluetooth,
        power_profile,
        now_playing,
        kbd_backlight,
        session: fixed(sill_model::session::Session::default(), &mut keep),
        locker: fixed(sill_model::locker::Lock::default(), &mut keep),
        notifications,
        events,
        screenshots,
        auth: fixed(sill_model::auth::Auth::default(), &mut keep),
        polkit_agent: fixed(sill_model::polkit_agent::Polkit::default(), &mut keep),
        clipboard: fixed(sill_model::clipboard::Clipboard::default(), &mut keep),
        idle,
        // Nowhere real: a surface that reads a file under these finds nothing.
        dirs: sill_services::env::Dirs::under(std::path::Path::new(
            "/nonexistent/sill-surfaces-test",
        )),
        session_actions: std::sync::Arc::new(sill_services::env::DryRunActions::new(
            sill_services::env::DryRunOffers::Everything,
            None,
        )),
    };
    let feeds = Feeds {
        clock: clock_io.state,
        battery: battery_io.state,
        network: network_io.state,
        volume: volume_io.state,
        tray: tray_io.state,
        cosmic_wl: cosmic_wl_io.state,
        wifi: wifi_io.state,
        bluetooth: bluetooth_io.state,
        power_profiles: profile_io.state,
        media: media_io.state,
        kbd_backlight: kbd_io.state,
        kbd_commands: kbd_io.commands,
        notifications: notifications_io.state,
        wl: cosmic_wl_io.commands,
        volume_commands: volume_io.commands,
        launch: launch_io.commands,
        wifi_commands: wifi_io.commands,
        bluetooth_commands: bluetooth_io.commands,
        profile_commands: profile_io.commands,
        media_commands: media_io.commands,
        notification_commands: notifications_io.commands,
        agenda: agenda_io.state,
        shots: shots_io.state,
        shot_commands: shots_io.commands,
        idle: idle_io.state,
        displays: displays_io.state,
        displays_commands: displays_io.commands,
        keep,
    };
    (services, feeds)
}
