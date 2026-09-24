//! Fake services for the bar's tests and `examples/bar_demo.rs`: every receiver is a `watch`
//! channel the caller holds the sender of, so a test feeds the frozen channels exactly as the
//! services will once they land.

#![allow(dead_code)] // Each test binary uses its own subset.

use shell_host::{OutputId, OutputInfo, Rect, Scale};
use sill_launcher::WindowKey;
use sill_services::{
    Activity, AppIndex, Badges, Battery, Charge, ClockText, Connectivity, Control, DockPins,
    GroupKey, Launches, Link, Mute, Network, OutputName, Percent, Presence, Results, Seconds,
    Services, Sizing, Toplevel, Toplevels, Tray, Urgency, Visibility, Volume, Wallpapers,
    WindowState, Workspace, WorkspaceGroup, WorkspaceKey, Workspaces,
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
    pub wl: mpsc::UnboundedReceiver<sill_services::WlCommand>,
    pub volume_commands: mpsc::UnboundedReceiver<sill_services::VolumeCommand>,
    pub launch: mpsc::UnboundedReceiver<sill_services::LaunchCommand>,
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
    let (wl, wl_rx) = Control::channel();
    let (volume_control, volume_rx) = Control::channel();
    let (launch, launch_rx) = Control::channel();
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
    };
    let feeds = Feeds {
        clock: clock_tx,
        battery: battery_tx,
        network: network_tx,
        volume: volume_tx,
        tray: tray_tx,
        workspaces: workspaces_tx,
        toplevels: toplevels_tx,
        wl: wl_rx,
        volume_commands: volume_rx,
        launch: launch_rx,
        keep,
    };
    (services, feeds)
}
