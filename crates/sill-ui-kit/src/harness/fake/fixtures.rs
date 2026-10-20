//! The fixed values the fake services start at, and the outputs and workspaces the surfaces are
//! drawn on.

use shell_host::{OutputId, OutputInfo, Rect, Scale120};
use sill_launcher::WindowKey;
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{
    Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower,
};
use sill_model::clock::ClockMinute;
use sill_model::desktop::{
    AppLifeKind, AppView, AppVisibility, Attention, DesktopView, ForceQuitRow, ProofKind, Standing,
};
use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};
use sill_model::network::{Connectivity, Link, Network};
use sill_model::now_playing::Media;
use sill_model::power_profile::{PowerProfiles, Profile, ProfileDegraded};
use sill_model::units::{Percent, Presence, Seconds, Urgency, Visibility};
use sill_model::volume::{Mute, Volume};
use sill_model::wifi::{AccessPoint, Association, Radio, Security, Ssid, Wifi};
use sill_model::windows::{
    Activity, Pinning, Sizing, Toplevel, Toplevels, WindowState, Workspace, WorkspaceCaps,
    WorkspaceGroup, Workspaces,
};

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
        scale: Scale120::ONE,
        refresh_mhz: 60_000,
    }
}

/// A 1920 x 1080 output with id 7 and the name `name` (unnamed when `None`), for layer tests.
pub fn output_named(name: Option<&str>) -> OutputInfo {
    OutputInfo {
        id: OutputId(7),
        name: name.map(str::to_owned),
        description: None,
        logical: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: Scale120::ONE,
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
        identifier: String::new(),
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
pub fn clock() -> ClockMinute {
    ClockMinute(jiff::civil::date(2026, 9, 24).at(14, 5, 0, 0))
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
    use ds::base::time::stamp::Stamp;
    use sill_model::now_playing::{Abilities, MediaPlayer, Playback, PlayerId, Timeline, Track};
    use sill_model::units::Availability;
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

/// A desktop whose Force Quit panel lists `rows`, in the order given: one app each, running or not
/// responding as its row says.
pub fn desktop_listing(rows: &[ForceQuitRow]) -> DesktopView {
    let apps = rows
        .iter()
        .zip(0u32..)
        .map(|(row, mru)| AppView {
            key: row.app.clone(),
            proof: ProofKind::Claimed,
            life: match row.standing {
                Standing::Responding => AppLifeKind::Running,
                Standing::NotResponding => AppLifeKind::NotResponding,
            },
            attention: Attention::None,
            mru,
            visibility: AppVisibility::Shown,
        })
        .collect();
    DesktopView {
        apps,
        ..DesktopView::default()
    }
}
