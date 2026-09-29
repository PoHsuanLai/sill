//! Fake services: every receiver is a `watch` channel the caller holds the sender of, so a test
//! feeds the frozen channels exactly as the services publish, and reads the commands a surface sends.

use super::fixtures::{
    battery, bluetooth, clock, network, power_profiles, toplevels, volume, wifi, workspaces,
};
use crate::services::Services;
use sill_model::apps::AppIndex;
use sill_model::badges::Badges;
use sill_model::battery::Battery;
use sill_model::bluetooth::{Bluetooth, BluetoothCommand};
use sill_model::clock::ClockText;
use sill_model::cosmic_wl::CosmicWl;
use sill_model::launch::Launches;
use sill_model::network::Network;
use sill_model::now_playing::{Media, MediaCommand};
use sill_model::pins::DockPins;
use sill_model::power_profile::{PowerProfiles, ProfileCommand};
use sill_model::results::Results;
use sill_model::tray::Tray;
use sill_model::volume::Volume;
use sill_model::wallpaper::Wallpapers;
use sill_model::wifi::{Wifi, WifiCommand};
use sill_services::{Handle, Service};
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
    /// What the session service offers (nothing until a test feeds it), and what it is asked.
    pub session: watch::Sender<sill_model::session::Session>,
    pub session_commands: mpsc::UnboundedReceiver<sill_model::session::SessionCommand>,
    /// The polkit agent's request (none until a test feeds one), and what the prompt sends it.
    pub polkit: watch::Sender<sill_model::polkit_agent::Polkit>,
    pub polkit_commands: mpsc::UnboundedReceiver<sill_model::polkit_agent::PolkitCommand>,
    /// The prepared wallpapers (none until a test feeds some).
    pub wallpaper: watch::Sender<Wallpapers>,
    /// Receivers and command queues nobody reads here, kept so sends do not fail.
    _keep: Vec<Box<dyn std::any::Any>>,
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
    let (session, session_io) = Handle::channel(sill_model::session::Session::default());
    let (polkit_agent, polkit_io) = Handle::channel(sill_model::polkit_agent::Polkit::default());
    let (wallpaper, wallpaper_io) = Handle::channel(Wallpapers::default());
    keep.push(Box::new(wallpaper_io.commands));
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
        wallpaper,
        displays,
        wifi,
        bluetooth,
        power_profile,
        now_playing,
        kbd_backlight,
        session,
        locker: fixed(sill_model::locker::Lock::default(), &mut keep),
        notifications,
        events,
        screenshots,
        auth: fixed(sill_model::auth::Auth::default(), &mut keep),
        polkit_agent,
        clipboard: fixed(sill_model::clipboard::Clipboard::default(), &mut keep),
        idle,
        // Nowhere real: a surface that reads a file under these finds nothing.
        dirs: sill_services::env::Dirs::under(std::path::Path::new(
            "/nonexistent/sill-ui-kit-test",
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
        session: session_io.state,
        session_commands: session_io.commands,
        polkit: polkit_io.state,
        polkit_commands: polkit_io.commands,
        wallpaper: wallpaper_io.state,
        _keep: keep,
    };
    (services, feeds)
}
