//! Fake services: every receiver is a `watch` channel the caller holds the sender of, so a test
//! feeds the frozen channels exactly as the services publish, and reads the commands a surface sends.

use crate::services::Services;
use sill_model::apps::AppIndex;
use sill_model::badges::Badges;
use sill_model::battery::Battery;
use sill_model::bluetooth::{Bluetooth, BluetoothCommand};
use sill_model::clock::ClockMinute;
use sill_model::network::Network;
use sill_model::now_playing::{Media, MediaCommand};
use sill_model::pins::DockPins;
use sill_model::power_profile::{PowerProfiles, ProfileCommand};
use sill_model::tray::Tray;
use sill_model::volume::Volume;
use sill_model::wallpaper::Wallpapers;
use sill_model::wifi::{Wifi, WifiCommand};
use sill_model::windows::WindowModel;
use tokio::sync::{mpsc, watch};

/// The senders behind a fake [`Services`], and the command queues the bar writes to.
#[derive(Debug)]
pub struct Feeds {
    pub clock: watch::Sender<ClockMinute>,
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

/// Fake services at the fixed values the registry gives them, and their senders.
pub fn services() -> (Services, Feeds) {
    let (services, mut ios) = Services::fake();
    let mut keep: Vec<Box<dyn std::any::Any>> = Vec::new();
    let clock = ios.clock.take();
    let battery = ios.battery.take();
    let network = ios.network.take();
    let volume = ios.volume.take();
    let tray = ios.tray.take();
    let app_menu = ios.app_menu.take();
    let windows = ios.windows.take();
    let wifi = ios.wifi.take();
    let bluetooth = ios.bluetooth.take();
    let power_profile = ios.power_profile.take();
    // No player until a test or the demo feeds one (`media()`).
    let now_playing = ios.now_playing.take();
    let launch = ios.launch.take();
    let kbd_backlight = ios.kbd_backlight.take();
    let notifications = ios.notifications.take();
    // A feed for the calendar widgets and one for the screenshot thumbnail.
    let events = ios.events.take();
    let screenshots = ios.screenshots.take();
    let idle = ios.idle.take();
    let displays = ios.displays.take();
    let session = ios.session.take();
    let polkit_agent = ios.polkit_agent.take();
    let accounts_sheet = ios.accounts_sheet.take();
    let capture = ios.capture.take();
    let intents = ios.intents.take();
    let companion = ios.companion.take();
    let voice = ios.voice.take();
    let night_shift = ios.night_shift.take();
    let wallpaper = ios.wallpaper.take();
    let apps = ios.apps.take();
    let badges = ios.badges.take();
    let pins = ios.pins.take();
    keep.push(Box::new(wallpaper.commands));
    keep.push(Box::new(apps.commands));
    keep.push(Box::new(badges.commands));
    keep.push(Box::new(clock.commands));
    keep.push(Box::new(battery.commands));
    keep.push(Box::new(network.commands));
    keep.push(Box::new(tray.commands));
    keep.push(Box::new(idle.commands));
    // Every other service keeps its halves, so sends to it do not fail.
    ios.keep_unclaimed(&mut keep);
    let feeds = Feeds {
        clock: clock.state,
        battery: battery.state,
        network: network.state,
        volume: volume.state,
        tray: tray.state,
        app_menu: app_menu.state,
        app_menu_commands: app_menu.commands,
        windows: windows.state,
        wifi: wifi.state,
        bluetooth: bluetooth.state,
        power_profiles: power_profile.state,
        media: now_playing.state,
        kbd_backlight: kbd_backlight.state,
        kbd_commands: kbd_backlight.commands,
        notifications: notifications.state,
        wl: windows.commands,
        volume_commands: volume.commands,
        launch: launch.commands,
        wifi_commands: wifi.commands,
        bluetooth_commands: bluetooth.commands,
        profile_commands: power_profile.commands,
        media_commands: now_playing.commands,
        notification_commands: notifications.commands,
        agenda: events.state,
        shots: screenshots.state,
        shot_commands: screenshots.commands,
        idle: idle.state,
        displays: displays.state,
        displays_commands: displays.commands,
        session: session.state,
        session_commands: session.commands,
        polkit: polkit_agent.state,
        polkit_commands: polkit_agent.commands,
        accounts_sheet: accounts_sheet.state,
        accounts_sheet_commands: accounts_sheet.commands,
        capture: capture.state,
        capture_commands: capture.commands,
        intents: intents.state,
        intents_commands: intents.commands,
        companion: companion.state,
        companion_commands: companion.commands,
        voice: voice.state,
        voice_commands: voice.commands,
        night_shift: night_shift.state,
        night_shift_commands: night_shift.commands,
        wallpaper: wallpaper.state,
        apps: apps.state,
        pins: pins.state,
        badges: badges.state,
        pin_commands: pins.commands,
        _keep: keep,
    };
    (services, feeds)
}
