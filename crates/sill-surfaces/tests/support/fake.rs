//! Fake services for the bar's tests and `examples/bar_demo.rs`: every receiver is a `watch`
//! channel the caller holds the sender of, so a test feeds the frozen channels exactly as the
//! services will once they land.

#![allow(dead_code)] // Each test binary uses its own subset.

use shell_host::{OutputId, OutputInfo, Rect, Scale};
use sill_launcher::WindowKey;
use sill_services::{
    AccessPoint, Activity, AppIndex, Association, Badges, Battery, Bluetooth, BluetoothCommand,
    BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower, Charge, ClockText,
    Connectivity, Control, DockPins, GroupKey, Launches, Link, Media, MediaCommand, Mute, Network,
    OutputName, Percent, Pinning, PowerProfiles, Presence, Profile, ProfileBus, ProfileCommand,
    ProfileDegraded, Radio, Results, Seconds, Security, Services, Sizing, Ssid, Toplevel,
    Toplevels, Tray, Urgency, Visibility, Volume, Wallpapers, Wifi, WifiCommand, WindowState,
    Workspace, WorkspaceCaps, WorkspaceGroup, WorkspaceKey, Workspaces,
};
use tokio::sync::{mpsc, watch};

/// The senders behind a fake [`Services`], and the command queues the bar writes to.
#[derive(Debug)]
pub struct Feeds {
    pub clock: watch::Sender<ClockText>,
    pub battery: watch::Sender<Battery>,
    pub network: watch::Sender<Network>,
    pub volume: watch::Sender<Volume>,
    pub tray: watch::Sender<Tray>,
    pub workspaces: watch::Sender<Workspaces>,
    pub toplevels: watch::Sender<Toplevels>,
    pub wifi: watch::Sender<Wifi>,
    pub bluetooth: watch::Sender<Bluetooth>,
    pub power_profiles: watch::Sender<PowerProfiles>,
    pub media: watch::Sender<Media>,
    /// M6: the live banners and the history (both empty until a test feeds them).
    pub notifications: watch::Sender<sill_services::Notifications>,
    pub notification_history: watch::Sender<sill_services::History>,
    pub wl: mpsc::UnboundedReceiver<sill_services::WlCommand>,
    pub volume_commands: mpsc::UnboundedReceiver<sill_services::VolumeCommand>,
    pub launch: mpsc::UnboundedReceiver<sill_services::LaunchCommand>,
    pub wifi_commands: mpsc::UnboundedReceiver<WifiCommand>,
    pub bluetooth_commands: mpsc::UnboundedReceiver<BluetoothCommand>,
    pub profile_commands: mpsc::UnboundedReceiver<ProfileCommand>,
    pub media_commands: mpsc::UnboundedReceiver<MediaCommand>,
    pub notification_commands: mpsc::UnboundedReceiver<sill_services::NotificationsCommand>,
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
    }
}

/// power-profiles-daemon on Balanced, all three offered.
pub fn power_profiles() -> PowerProfiles {
    PowerProfiles {
        presence: Presence::Present,
        bus: ProfileBus::UPower,
        active: Profile::Balanced,
        offered: Profile::ALL.to_vec(),
        degraded: ProfileDegraded::No,
    }
}

/// One player, playing "So What" (no art file), every button live.
pub fn media() -> Media {
    use sill_services::{
        Abilities, Availability, MediaPlayer, Playback, PlayerId, Stamp, Timeline, Track,
    };
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

/// A receiver whose sender is kept in `keep`.
fn fixed<T: 'static>(value: T, keep: &mut Vec<Box<dyn std::any::Any>>) -> watch::Receiver<T> {
    let (tx, rx) = watch::channel(value);
    keep.push(Box::new(tx));
    rx
}

/// A control whose queue is kept in `keep`.
fn control<C: 'static>(keep: &mut Vec<Box<dyn std::any::Any>>) -> Control<C> {
    let (control, rx) = Control::channel();
    keep.push(Box::new(rx));
    control
}

/// Fake services at the fixed values above, and their senders.
pub fn services() -> (Services, Feeds) {
    let mut keep: Vec<Box<dyn std::any::Any>> = Vec::new();
    let (clock_tx, clock) = watch::channel(clock());
    let (battery_tx, battery) = watch::channel(battery());
    let (network_tx, network) = watch::channel(network());
    let (volume_tx, volume) = watch::channel(volume());
    let (tray_tx, tray) = watch::channel(Tray::default());
    let (workspaces_tx, workspaces) = watch::channel(workspaces());
    let (toplevels_tx, toplevels) = watch::channel(toplevels());
    let (wifi_tx, wifi) = watch::channel(wifi());
    let (bluetooth_tx, bluetooth) = watch::channel(bluetooth());
    let (profiles_tx, power_profiles) = watch::channel(power_profiles());
    // No player until a test or the demo feeds one (`media()`).
    let (media_tx, media) = watch::channel(Media::default());
    let (wl, wl_rx) = Control::channel();
    let (volume_control, volume_rx) = Control::channel();
    let (launch, launch_rx) = Control::channel();
    let (wifi_control, wifi_rx) = Control::channel();
    let (bluetooth_control, bluetooth_rx) = Control::channel();
    let (power_profile_control, profile_rx) = Control::channel();
    let (media_control, media_rx) = Control::channel();
    let (notifications_tx, notifications) = watch::channel(sill_services::Notifications::default());
    let (history_tx, notification_history) = watch::channel(sill_services::History::default());
    let (notifications_control, notification_rx) = Control::channel();
    let services = Services {
        clock,
        clock_control: control(&mut keep),
        battery,
        battery_control: control(&mut keep),
        network,
        network_control: control(&mut keep),
        volume,
        volume_control,
        tray,
        tray_control: control(&mut keep),
        workspaces,
        toplevels,
        wl,
        apps: fixed(AppIndex::default(), &mut keep),
        apps_control: control(&mut keep),
        badges: fixed(Badges::default(), &mut keep),
        badges_control: control(&mut keep),
        pins: fixed(DockPins::default(), &mut keep),
        pins_control: control(&mut keep),
        launches: fixed(Launches::default(), &mut keep),
        launch,
        results: fixed(Results::default(), &mut keep),
        search: control(&mut keep),
        wallpapers: fixed(Wallpapers::default(), &mut keep),
        wallpaper_control: control(&mut keep),
        sound: fixed(Presence::Absent, &mut keep),
        sound_control: control(&mut keep),
        displays: fixed(sill_services::Displays::default(), &mut keep),
        displays_control: control(&mut keep),
        wifi,
        wifi_control,
        bluetooth,
        bluetooth_control,
        power_profiles,
        power_profile_control,
        media,
        media_control,
        session: fixed(sill_services::Session::default(), &mut keep),
        session_control: control(&mut keep),
        notifications,
        notification_history,
        notifications_control,
    };
    let feeds = Feeds {
        clock: clock_tx,
        battery: battery_tx,
        network: network_tx,
        volume: volume_tx,
        tray: tray_tx,
        workspaces: workspaces_tx,
        toplevels: toplevels_tx,
        wifi: wifi_tx,
        bluetooth: bluetooth_tx,
        power_profiles: profiles_tx,
        media: media_tx,
        notifications: notifications_tx,
        notification_history: history_tx,
        wl: wl_rx,
        volume_commands: volume_rx,
        launch: launch_rx,
        wifi_commands: wifi_rx,
        bluetooth_commands: bluetooth_rx,
        profile_commands: profile_rx,
        media_commands: media_rx,
        notification_commands: notification_rx,
        keep,
    };
    (services, feeds)
}
