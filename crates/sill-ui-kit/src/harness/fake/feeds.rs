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
use sill_model::windows::WindowModel;
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
    /// The focused apps' menus (none until a test feeds some), and what the bar tells the service.
    pub app_menu: watch::Sender<sill_model::app_menu::AppMenus>,
    pub app_menu_commands: mpsc::UnboundedReceiver<sill_model::app_menu::AppMenuCommand>,
    pub windows: watch::Sender<WindowModel>,
    pub wifi: watch::Sender<Wifi>,
    pub bluetooth: watch::Sender<Bluetooth>,
    pub power_profiles: watch::Sender<PowerProfiles>,
    pub media: watch::Sender<Media>,
    /// The keyboard backlight: absent until a test feeds one (`kbd_backlight()`).
    pub kbd_backlight: watch::Sender<sill_model::kbd_backlight::KbdBacklight>,
    pub kbd_commands: mpsc::UnboundedReceiver<sill_model::kbd_backlight::KbdCommand>,
    /// The live banners and the history (both empty until a test feeds them).
    pub notifications: watch::Sender<sill_model::notifications::NotesState>,
    pub wl: mpsc::UnboundedReceiver<sill_model::windows::WlCommand>,
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
    /// intentd's seat: the confirmations waiting and the terminal's grants (none until a test feeds
    /// them), and what the sheet and the control center send it.
    pub accounts_sheet: watch::Sender<sill_model::accounts::sheet::AccountsSheets>,
    pub accounts_sheet_commands:
        mpsc::UnboundedReceiver<sill_model::accounts::sheet::AccountsSheetCommand>,
    pub capture: watch::Sender<sill_model::capture_pick::CapturePicker>,
    pub capture_commands: mpsc::UnboundedReceiver<sill_model::capture_pick::CaptureCommand>,
    pub intents: watch::Sender<sill_model::intents::Intents>,
    pub intents_commands: mpsc::UnboundedReceiver<sill_model::intents::IntentsCommand>,
    /// The companion's answers, runs and journal (none until a test feeds them), and what the
    /// launcher and the surfaces send it.
    pub companion: watch::Sender<sill_model::companion::Companion>,
    pub companion_commands: mpsc::UnboundedReceiver<sill_model::companion::CompanionCommand>,
    /// Hold-to-talk: where the hold stands (idle until a test feeds it), and what the prompt and
    /// the consent sheet send it.
    pub voice: watch::Sender<sill_model::voice::Voice>,
    pub voice_commands: mpsc::UnboundedReceiver<sill_model::voice::VoiceCommand>,
    /// The prepared wallpapers (none until a test feeds some).
    pub wallpaper: watch::Sender<Wallpapers>,
    /// Night Shift: unreachable until a test feeds it, and what the control center sends it.
    pub night_shift: watch::Sender<sill_model::night_shift::NightShift>,
    pub night_shift_commands: mpsc::UnboundedReceiver<sill_model::night_shift::NightShiftCommand>,
    /// The dock's app index, pins and badges (empty until a test feeds them), and the pin
    /// commands it sends.
    pub apps: watch::Sender<AppIndex>,
    pub pins: watch::Sender<DockPins>,
    pub badges: watch::Sender<Badges>,
    pub pin_commands: mpsc::UnboundedReceiver<sill_model::pins::PinEvent>,
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
    let (app_menu, app_menu_io) = Handle::channel(sill_model::app_menu::AppMenus::default());
    let (windows, windows_io) = Handle::channel(WindowModel {
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
    let (accounts_sheet, accounts_sheet_io) =
        Handle::channel(sill_model::accounts::sheet::AccountsSheets::default());
    let (capture, capture_io) = Handle::channel(sill_model::capture_pick::CapturePicker::default());
    let (intents, intents_io) = Handle::channel(sill_model::intents::Intents::default());
    let (companion, companion_io) = Handle::channel(sill_model::companion::Companion::default());
    let (voice, voice_io) = Handle::channel(sill_model::voice::Voice::default());
    let (night_shift, night_shift_io) =
        Handle::channel(sill_model::night_shift::NightShift::default());
    let (wallpaper, wallpaper_io) = Handle::channel(Wallpapers::default());
    keep.push(Box::new(wallpaper_io.commands));
    let (apps, apps_io) = Handle::channel(AppIndex::default());
    let (badges, badges_io) = Handle::channel(Badges::default());
    let (pins, pins_io) = Handle::channel(DockPins::default());
    keep.push(Box::new(apps_io.commands));
    keep.push(Box::new(badges_io.commands));
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
        app_menu,
        windows,
        apps,
        badges,
        pins,
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
        keycap: fixed(sill_model::keycap::KeycapState::default(), &mut keep),
        accounts: fixed(sill_model::accounts::AccountsState::default(), &mut keep),
        accounts_sheet,
        sync_hold: fixed(sill_model::sync_hold::SyncHoldState::default(), &mut keep),
        guest_asks: fixed(sill_model::guest_asks::GuestAsksState::default(), &mut keep),
        work_areas: fixed(sill_model::work_areas::WorkAreas::default(), &mut keep),
        night_shift,
        workspace_looks: fixed(
            sill_model::workspace_looks::WorkspaceLooks::default(),
            &mut keep,
        ),
        capture,
        intents,
        companion,
        voice,
        dock_rects: crate::dock_rects::DockRects::default(),
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
        app_menu: app_menu_io.state,
        app_menu_commands: app_menu_io.commands,
        windows: windows_io.state,
        wifi: wifi_io.state,
        bluetooth: bluetooth_io.state,
        power_profiles: profile_io.state,
        media: media_io.state,
        kbd_backlight: kbd_io.state,
        kbd_commands: kbd_io.commands,
        notifications: notifications_io.state,
        wl: windows_io.commands,
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
        accounts_sheet: accounts_sheet_io.state,
        accounts_sheet_commands: accounts_sheet_io.commands,
        capture: capture_io.state,
        capture_commands: capture_io.commands,
        intents: intents_io.state,
        intents_commands: intents_io.commands,
        companion: companion_io.state,
        companion_commands: companion_io.commands,
        voice: voice_io.state,
        voice_commands: voice_io.commands,
        night_shift: night_shift_io.state,
        night_shift_commands: night_shift_io.commands,
        wallpaper: wallpaper_io.state,
        apps: apps_io.state,
        pins: pins_io.state,
        badges: badges_io.state,
        pin_commands: pins_io.commands,
        _keep: keep,
    };
    (services, feeds)
}
