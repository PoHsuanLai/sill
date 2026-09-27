//! The command line (plan: `sill run|launcher {toggle,open [--query],close}|dock {pin,unpin}|
//! reload|quit|ping|debug {stats,pointer,leave,capture}`, plus `debug click` from the M2 freeze
//! amendment, `display {brightness,volume}` from the displays pass (`cli_display`), and
//! `power-menu {open,close,toggle}` and `control-center toggle` from the M5 freeze,
//! `notifications {toggle,open,close,dismiss-all,status}` and `debug notify` from the M6 freeze
//! (`cli_notifications`), `lock` and `polkit-agent` from M7, `screenshot`, `widgets
//! toggle-desktop`, `debug screenshot` and `debug corner` from the M10 freeze (`cli_desktop`),
//! `switcher`, `lock status`, `polkit status`, `debug auth`, `debug polkit`, `debug switcher`
//! and the hidden `auth-check` helper from the M11 freeze (`cli_session`), and its one pure
//! mapping to an IPC request.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub use crate::cli_desktop::{
    CornerArg, ShotArg, WidgetsAction, fake_corner, fake_screenshot, screenshot_request,
    widgets_request,
};
pub use crate::cli_display::{DisplayAction, LevelAction, VolumeAction, display_request};
pub use crate::cli_launcher::{ClipboardAction, LauncherKeyArg, clipboard_request, launcher_key};
pub use crate::cli_notifications::{
    ExpireArg, FakeNotifyArgs, NotificationsAction, UrgencyArg, fake_notification,
    notifications_request,
};
pub use crate::cli_session::{
    FakeAuthArg, FakePolkitArgs, IdleAction, LockVerb, PolkitAction, SwitcherAction, fake_auth,
    fake_polkit, fake_switcher, idle_request, lock_request, polkit_request, switcher_request,
};
use sill_ipc::{
    ControlCenterRequest, DebugRequest, DesktopId, DockRequest, FakeMute, FakeOsd, LauncherRequest,
    OsdTopic, Point, PointerButton, PowerMenuRequest, Request, SurfaceName,
};

/// sill, the COSMIC shell.
#[derive(Debug, Parser)]
#[command(name = "sill", version, about)]
pub struct Cli {
    /// Write `sill.settings.toml` (the settings schema, design/22 §9.2) into DIR and exit.
    #[arg(long, value_name = "DIR")]
    pub write_schema: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Run the shell (the daemon); exits 0 if one is already running.
    Run,
    /// Show, hide or seed the launcher.
    Launcher {
        #[command(subcommand)]
        action: LauncherAction,
    },
    /// Pin or unpin a dock app.
    Dock {
        #[command(subcommand)]
        action: DockAction,
    },
    /// Re-read settings, apps and pins.
    Reload,
    /// Stop the daemon.
    Quit,
    /// Is the daemon alive?
    Ping,
    /// Development probes.
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
    /// The brightness and volume keys.
    Display {
        #[command(subcommand)]
        action: DisplayAction,
    },
    /// Show or hide the power menu (Log Out, Restart, Shut Down, Suspend).
    PowerMenu {
        #[command(subcommand)]
        action: PowerMenuAction,
    },
    /// Open or close the control center.
    ControlCenter {
        #[command(subcommand)]
        action: ControlCenterAction,
    },
    /// The notification center and the banners.
    Notifications {
        #[command(subcommand)]
        action: NotificationsAction,
    },
    /// Lock the screen now with the configured locker (`session.locker`), or say what it is
    /// doing.
    Lock {
        #[arg(value_enum, default_value_t = LockVerb::Now)]
        verb: LockVerb,
    },
    /// The app switcher's chord (COSMIC's window switcher shortcuts run these).
    Switcher {
        #[command(subcommand)]
        action: SwitcherAction,
    },
    /// sill's own polkit agent.
    Polkit {
        #[command(subcommand)]
        action: PolkitAction,
    },
    /// sill's own idle: the dim, the displays off, the lock at screen off and the idle suspend.
    Idle {
        #[command(subcommand)]
        action: IdleAction,
    },
    /// The launcher's clipboard history (M9).
    Clipboard {
        #[command(subcommand)]
        action: ClipboardAction,
    },
    /// The lock screen's PAM helper: reads one request on stdin, checks it, answers on stdout
    /// (the daemon runs it; not for people).
    #[command(hide = true)]
    AuthCheck,
    /// Start the polkit agent `session.polkit_agent` names (dist/sill-session runs this), or
    /// print which one it would be.
    PolkitAgent {
        #[arg(value_enum, default_value_t = AgentVerb::Exec)]
        verb: AgentVerb,
    },
    /// Take a screenshot with `screenshot.tool`; its thumbnail shows at the bottom right.
    Screenshot {
        #[arg(value_enum, default_value_t = ShotArg::Area)]
        kind: ShotArg,
    },
    /// The widgets.
    Widgets {
        #[command(subcommand)]
        action: WidgetsAction,
    },
}

/// `sill polkit-agent [exec|print]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentVerb {
    /// Replace this process with the agent.
    Exec,
    /// Print the agent's command line (nothing when none) and exit.
    Print,
}

/// `sill power-menu ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum PowerMenuAction {
    Open,
    Close,
    Toggle,
}

/// `sill control-center ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ControlCenterAction {
    Toggle,
}

/// `sill launcher ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum LauncherAction {
    Toggle,
    Open {
        /// Text to start with.
        #[arg(long, default_value = "")]
        query: String,
    },
    Close,
}

/// `sill dock ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DockAction {
    /// Keep APP (a desktop id, `org.mozilla.firefox`) in the dock.
    Pin {
        app: String,
    },
    Unpin {
        app: String,
    },
}

/// `sill debug ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DebugAction {
    /// Frame statistics for every surface.
    Stats,
    /// Move the pointer to X Y on SURFACE (`sill-dock-DP-1`).
    Pointer { surface: String, x: i32, y: i32 },
    /// The pointer leaves SURFACE.
    Leave { surface: String },
    /// Render SURFACE to a PNG at PATH.
    Capture { surface: String, path: PathBuf },
    /// Press and release a pointer button at X Y on SURFACE (`sill-bar-DP-1`).
    Click {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Show the OSD for a made-up change, moving no real level.
    Osd {
        /// What changed.
        #[arg(value_enum)]
        topic: FakeTopic,
        /// The level to show, 0-100.
        percent: u8,
        /// Whether the made-up volume is muted.
        #[arg(long, value_enum, default_value_t = FakeMuteArg::Audible)]
        mute: FakeMuteArg,
    },
    /// Post a made-up notification without D-Bus: a banner, and a row in the center.
    Notify(FakeNotifyArgs),
    /// Show the screenshot thumbnail for PATH, as if it had just been saved.
    Screenshot { path: PathBuf },
    /// Carry out CORNER's hot corner action, as if the pointer had rested there.
    Corner {
        #[arg(value_enum)]
        corner: CornerArg,
    },
    /// `button` presses at X Y on SURFACE and stays down: the first half of a real drag
    /// (shell-host's `Inject::PointerDown`, G330 closed). `sill debug pointer` moves while it
    /// is held, and `pointer-release` lets go.
    PointerPress {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// `button` releases at X Y on SURFACE: the second half of a real drag
    /// (`Inject::PointerUp`).
    PointerRelease {
        surface: String,
        x: i32,
        y: i32,
        /// Which button.
        #[arg(long, value_enum, default_value_t = ClickButton::Primary)]
        button: ClickButton,
    },
    /// Feed the lock screen's password check a made-up verdict (a debug daemon only).
    Auth {
        #[arg(value_enum)]
        verdict: FakeAuthArg,
    },
    /// Show a made-up polkit request (its stand-in helper accepts `sill-fake`).
    Polkit(FakePolkitArgs),
    /// Open the app switcher on made-up apps (desktop ids, the current one first).
    Switcher {
        #[arg(required = true)]
        apps: Vec<String>,
    },
    /// Press a key on the open launcher (a debug daemon only; M9).
    LauncherKey {
        #[arg(value_enum)]
        key: LauncherKeyArg,
    },
    /// Print the open launcher's sections, rows and preview (a debug daemon only; M9).
    LauncherRows,
    /// Wake idle as real input would (a debug daemon only; inside the lock grace it opens the
    /// lock).
    IdleWake,
}

/// `sill debug osd TOPIC`: the command line's spelling of [`OsdTopic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FakeTopic {
    Volume,
    Brightness,
}

/// `sill debug osd --mute ...`: the command line's spelling of [`FakeMute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FakeMuteArg {
    Audible,
    Muted,
}

/// The fake change `sill debug osd` asks for.
fn fake_osd(topic: FakeTopic, percent: u8, mute: FakeMuteArg) -> FakeOsd {
    FakeOsd {
        topic: match topic {
            FakeTopic::Volume => OsdTopic::Volume,
            FakeTopic::Brightness => OsdTopic::Brightness,
        },
        percent,
        mute: match mute {
            FakeMuteArg::Audible => FakeMute::Audible,
            FakeMuteArg::Muted => FakeMute::Muted,
        },
    }
}

/// `sill debug click --button ...`: the command line's spelling of [`PointerButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClickButton {
    Primary,
    Secondary,
    Middle,
}

impl From<ClickButton> for PointerButton {
    fn from(button: ClickButton) -> Self {
        match button {
            ClickButton::Primary => PointerButton::Primary,
            ClickButton::Secondary => PointerButton::Secondary,
            ClickButton::Middle => PointerButton::Middle,
        }
    }
}

/// The request `command` sends; `None` for `run`, which is not a request.
pub fn request(command: &Command) -> Option<Request> {
    Some(match command {
        Command::Run | Command::PolkitAgent { .. } | Command::AuthCheck => return None,
        Command::Lock { verb } => lock_request(*verb),
        Command::Switcher { action } => switcher_request(*action),
        Command::Polkit { action } => polkit_request(*action),
        Command::Idle { action } => idle_request(*action),
        Command::Clipboard { action } => clipboard_request(*action),
        Command::Launcher { action } => Request::Launcher(match action {
            LauncherAction::Toggle => LauncherRequest::Toggle,
            LauncherAction::Open { query } => LauncherRequest::Open {
                query: query.clone(),
            },
            LauncherAction::Close => LauncherRequest::Close,
        }),
        Command::Dock { action } => Request::Dock(match action {
            DockAction::Pin { app } => DockRequest::Pin {
                app: DesktopId(app.clone()),
            },
            DockAction::Unpin { app } => DockRequest::Unpin {
                app: DesktopId(app.clone()),
            },
        }),
        Command::Reload => Request::Reload,
        Command::Quit => Request::Quit,
        Command::Ping => Request::Ping,
        Command::Display { action } => Request::Display(display_request(action)),
        Command::PowerMenu { action } => Request::PowerMenu(match action {
            PowerMenuAction::Open => PowerMenuRequest::Open,
            PowerMenuAction::Close => PowerMenuRequest::Close,
            PowerMenuAction::Toggle => PowerMenuRequest::Toggle,
        }),
        Command::ControlCenter { action } => Request::ControlCenter(match action {
            ControlCenterAction::Toggle => ControlCenterRequest::Toggle,
        }),
        Command::Notifications { action } => Request::Notifications(notifications_request(*action)),
        Command::Screenshot { kind } => Request::Screenshot(screenshot_request(*kind)),
        Command::Widgets { action } => Request::Widgets(widgets_request(*action)),
        Command::Debug { action } => Request::Debug(match action {
            DebugAction::Stats => DebugRequest::Stats,
            DebugAction::Pointer { surface, x, y } => DebugRequest::Pointer {
                surface: SurfaceName(surface.clone()),
                at: Point { x: *x, y: *y },
            },
            DebugAction::Leave { surface } => DebugRequest::Leave {
                surface: SurfaceName(surface.clone()),
            },
            DebugAction::Capture { surface, path } => DebugRequest::Capture {
                surface: SurfaceName(surface.clone()),
                path: path.clone(),
            },
            DebugAction::Click {
                surface,
                x,
                y,
                button,
            } => DebugRequest::Click {
                surface: SurfaceName(surface.clone()),
                at: Point { x: *x, y: *y },
                button: (*button).into(),
            },
            DebugAction::Osd {
                topic,
                percent,
                mute,
            } => DebugRequest::Osd(fake_osd(*topic, *percent, *mute)),
            DebugAction::Notify(args) => DebugRequest::Notify(fake_notification(args)),
            DebugAction::Screenshot { path } => {
                DebugRequest::Screenshot(fake_screenshot(path.clone()))
            }
            DebugAction::Corner { corner } => DebugRequest::Corner(fake_corner(*corner)),
            DebugAction::PointerPress {
                surface,
                x,
                y,
                button,
            } => DebugRequest::PointerPress {
                surface: SurfaceName(surface.clone()),
                at: Point { x: *x, y: *y },
                button: (*button).into(),
            },
            DebugAction::PointerRelease {
                surface,
                x,
                y,
                button,
            } => DebugRequest::PointerRelease {
                surface: SurfaceName(surface.clone()),
                at: Point { x: *x, y: *y },
                button: (*button).into(),
            },
            DebugAction::Auth { verdict } => DebugRequest::Auth(fake_auth(*verdict)),
            DebugAction::Polkit(args) => DebugRequest::Polkit(fake_polkit(args)),
            DebugAction::Switcher { apps } => DebugRequest::Switcher(fake_switcher(apps)),
            DebugAction::LauncherKey { key } => DebugRequest::LauncherKey(launcher_key(*key)),
            DebugAction::LauncherRows => DebugRequest::LauncherRows,
            DebugAction::IdleWake => DebugRequest::IdleWake,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::{Cli, request};
    use clap::Parser;
    use sill_ipc::{
        ControlCenterRequest, DebugRequest, DesktopId, DockRequest, FakeAction, FakeExpire,
        FakeMute, FakeNotification, FakeOsd, FakeUrgency, LauncherRequest, NotificationsRequest,
        OsdTopic, Point, PointerButton, PowerMenuRequest, Request, SurfaceName,
    };
    use sill_ipc::{
        CornerName, FakeCorner, FakeScreenshot, ScreenshotRequest, ShotArea, WidgetsRequest,
    };
    use sill_ipc::{
        FakeAuth, FakePolkit, FakeSwitcher, LockRequest, PolkitRequest, SwitcherRequest,
    };
    use std::path::PathBuf;

    #[test]
    fn every_command_line_sends_its_request() {
        let cases: Vec<(&[&str], Option<Request>)> = vec![
            (&["run"], None),
            (&["ping"], Some(Request::Ping)),
            (&["reload"], Some(Request::Reload)),
            (&["quit"], Some(Request::Quit)),
            (&["lock"], Some(Request::Lock)),
            (&["lock", "now"], Some(Request::Lock)),
            (
                &["lock", "status"],
                Some(Request::LockStatus(LockRequest::Status)),
            ),
            (&["auth-check"], None),
            (
                &["switcher", "next"],
                Some(Request::Switcher(SwitcherRequest::Next)),
            ),
            (
                &["switcher", "prev"],
                Some(Request::Switcher(SwitcherRequest::Prev)),
            ),
            (
                &["switcher", "cancel"],
                Some(Request::Switcher(SwitcherRequest::Cancel)),
            ),
            (
                &["switcher", "commit"],
                Some(Request::Switcher(SwitcherRequest::Commit)),
            ),
            (
                &["polkit", "status"],
                Some(Request::Polkit(PolkitRequest::Status)),
            ),
            (
                &["idle", "status"],
                Some(Request::Idle(sill_ipc::IdleRequest::Status)),
            ),
            (
                &["clipboard", "status"],
                Some(Request::Clipboard(sill_ipc::ClipboardRequest::Status)),
            ),
            (
                &["clipboard", "clear"],
                Some(Request::Clipboard(sill_ipc::ClipboardRequest::Clear)),
            ),
            (
                &["debug", "launcher-key", "quick-look"],
                Some(Request::Debug(DebugRequest::LauncherKey(
                    sill_ipc::FakeLauncherKey::QuickLook,
                ))),
            ),
            (
                &["debug", "launcher-rows"],
                Some(Request::Debug(DebugRequest::LauncherRows)),
            ),
            (
                &["debug", "idle-wake"],
                Some(Request::Debug(DebugRequest::IdleWake)),
            ),
            (
                &["debug", "auth", "accept"],
                Some(Request::Debug(DebugRequest::Auth(FakeAuth::Accept))),
            ),
            (
                &["debug", "auth", "refuse"],
                Some(Request::Debug(DebugRequest::Auth(FakeAuth::Refuse))),
            ),
            (
                &["debug", "polkit"],
                Some(Request::Debug(DebugRequest::Polkit(FakePolkit {
                    action_id: "org.sill.debug.prompt".to_owned(),
                    message: "sill is testing its password prompt".to_owned(),
                }))),
            ),
            (
                &[
                    "debug",
                    "polkit",
                    "--action",
                    "org.example.run",
                    "--message",
                    "Run?",
                ],
                Some(Request::Debug(DebugRequest::Polkit(FakePolkit {
                    action_id: "org.example.run".to_owned(),
                    message: "Run?".to_owned(),
                }))),
            ),
            (
                &["debug", "switcher", "foot", "firefox"],
                Some(Request::Debug(DebugRequest::Switcher(FakeSwitcher {
                    apps: vec!["foot".to_owned(), "firefox".to_owned()],
                }))),
            ),
            (&["polkit-agent"], None),
            (&["polkit-agent", "print"], None),
            (
                &["screenshot"],
                Some(Request::Screenshot(ScreenshotRequest::Take(ShotArea::Area))),
            ),
            (
                &["screenshot", "window"],
                Some(Request::Screenshot(ScreenshotRequest::Take(
                    ShotArea::Window,
                ))),
            ),
            (
                &["screenshot", "screen"],
                Some(Request::Screenshot(ScreenshotRequest::Take(
                    ShotArea::Screen,
                ))),
            ),
            (
                &["widgets", "toggle-desktop"],
                Some(Request::Widgets(WidgetsRequest::ToggleDesktop)),
            ),
            (
                &["widgets", "edit"],
                Some(Request::Widgets(WidgetsRequest::Edit)),
            ),
            (
                &["widgets", "edit-close"],
                Some(Request::Widgets(WidgetsRequest::EditClose)),
            ),
            (
                &["debug", "screenshot", "/tmp/shots/Screenshot_1.png"],
                Some(Request::Debug(DebugRequest::Screenshot(FakeScreenshot {
                    path: PathBuf::from("/tmp/shots/Screenshot_1.png"),
                }))),
            ),
            (
                &["debug", "corner", "top-left"],
                Some(Request::Debug(DebugRequest::Corner(FakeCorner {
                    corner: CornerName::TopLeft,
                }))),
            ),
            (
                &["debug", "corner", "bottom-right"],
                Some(Request::Debug(DebugRequest::Corner(FakeCorner {
                    corner: CornerName::BottomRight,
                }))),
            ),
            (
                &["power-menu", "open"],
                Some(Request::PowerMenu(PowerMenuRequest::Open)),
            ),
            (
                &["power-menu", "close"],
                Some(Request::PowerMenu(PowerMenuRequest::Close)),
            ),
            (
                &["power-menu", "toggle"],
                Some(Request::PowerMenu(PowerMenuRequest::Toggle)),
            ),
            (
                &["control-center", "toggle"],
                Some(Request::ControlCenter(ControlCenterRequest::Toggle)),
            ),
            (
                &["notifications", "toggle"],
                Some(Request::Notifications(NotificationsRequest::Toggle)),
            ),
            (
                &["notifications", "open"],
                Some(Request::Notifications(NotificationsRequest::Open)),
            ),
            (
                &["notifications", "close"],
                Some(Request::Notifications(NotificationsRequest::Close)),
            ),
            (
                &["notifications", "dismiss-all"],
                Some(Request::Notifications(NotificationsRequest::DismissAll)),
            ),
            (
                &["notifications", "status"],
                Some(Request::Notifications(NotificationsRequest::Status)),
            ),
            (
                &["debug", "notify", "Hello"],
                Some(Request::Debug(DebugRequest::Notify(FakeNotification {
                    app: "sill".to_owned(),
                    summary: "Hello".to_owned(),
                    body: String::new(),
                    urgency: FakeUrgency::Normal,
                    actions: Vec::new(),
                    expire: FakeExpire::Default,
                    replaces: 0,
                }))),
            ),
            (
                &[
                    "debug",
                    "notify",
                    "Ada",
                    "--app",
                    "Mail",
                    "--body",
                    "the <b>engine</b>",
                    "--urgency",
                    "critical",
                    "--action",
                    "default=Open",
                    "--action",
                    "reply=Reply",
                    "--expire",
                    "never",
                    "--replaces",
                    "4",
                ],
                Some(Request::Debug(DebugRequest::Notify(FakeNotification {
                    app: "Mail".to_owned(),
                    summary: "Ada".to_owned(),
                    body: "the <b>engine</b>".to_owned(),
                    urgency: FakeUrgency::Critical,
                    actions: vec![
                        FakeAction {
                            key: "default".to_owned(),
                            label: "Open".to_owned(),
                        },
                        FakeAction {
                            key: "reply".to_owned(),
                            label: "Reply".to_owned(),
                        },
                    ],
                    expire: FakeExpire::Never,
                    replaces: 4,
                }))),
            ),
            (
                &["launcher", "toggle"],
                Some(Request::Launcher(LauncherRequest::Toggle)),
            ),
            (
                &["launcher", "close"],
                Some(Request::Launcher(LauncherRequest::Close)),
            ),
            (
                &["launcher", "open"],
                Some(Request::Launcher(LauncherRequest::Open {
                    query: String::new(),
                })),
            ),
            (
                &["launcher", "open", "--query", ""],
                Some(Request::Launcher(LauncherRequest::Open {
                    query: String::new(),
                })),
            ),
            (
                &["launcher", "open", "--query", "fire"],
                Some(Request::Launcher(LauncherRequest::Open {
                    query: "fire".to_owned(),
                })),
            ),
            (
                &["dock", "pin", "foot"],
                Some(Request::Dock(DockRequest::Pin {
                    app: DesktopId("foot".to_owned()),
                })),
            ),
            (
                &["dock", "unpin", "foot"],
                Some(Request::Dock(DockRequest::Unpin {
                    app: DesktopId("foot".to_owned()),
                })),
            ),
            (
                &["debug", "stats"],
                Some(Request::Debug(DebugRequest::Stats)),
            ),
            (
                &["debug", "pointer", "sill-dock-DP-1", "300", "64"],
                Some(Request::Debug(DebugRequest::Pointer {
                    surface: SurfaceName("sill-dock-DP-1".to_owned()),
                    at: Point { x: 300, y: 64 },
                })),
            ),
            (
                &["debug", "leave", "sill-dock-DP-1"],
                Some(Request::Debug(DebugRequest::Leave {
                    surface: SurfaceName("sill-dock-DP-1".to_owned()),
                })),
            ),
            (
                &["debug", "capture", "sill-bar-DP-1", "/tmp/bar.png"],
                Some(Request::Debug(DebugRequest::Capture {
                    surface: SurfaceName("sill-bar-DP-1".to_owned()),
                    path: PathBuf::from("/tmp/bar.png"),
                })),
            ),
            (
                &["debug", "click", "sill-bar-DP-1", "1200", "16"],
                Some(Request::Debug(DebugRequest::Click {
                    surface: SurfaceName("sill-bar-DP-1".to_owned()),
                    at: Point { x: 1200, y: 16 },
                    button: PointerButton::Primary,
                })),
            ),
            (
                &[
                    "debug",
                    "click",
                    "sill-dock-DP-1",
                    "40",
                    "30",
                    "--button",
                    "secondary",
                ],
                Some(Request::Debug(DebugRequest::Click {
                    surface: SurfaceName("sill-dock-DP-1".to_owned()),
                    at: Point { x: 40, y: 30 },
                    button: PointerButton::Secondary,
                })),
            ),
            (
                &["debug", "click", "s", "0", "0", "--button", "middle"],
                Some(Request::Debug(DebugRequest::Click {
                    surface: SurfaceName("s".to_owned()),
                    at: Point { x: 0, y: 0 },
                    button: PointerButton::Middle,
                })),
            ),
            (
                &[
                    "debug",
                    "pointer-press",
                    "sill-desktop-widgets-DP-1",
                    "900",
                    "96",
                ],
                Some(Request::Debug(DebugRequest::PointerPress {
                    surface: SurfaceName("sill-desktop-widgets-DP-1".to_owned()),
                    at: Point { x: 900, y: 96 },
                    button: PointerButton::Primary,
                })),
            ),
            (
                &[
                    "debug",
                    "pointer-press",
                    "sill-dock-DP-1",
                    "40",
                    "30",
                    "--button",
                    "secondary",
                ],
                Some(Request::Debug(DebugRequest::PointerPress {
                    surface: SurfaceName("sill-dock-DP-1".to_owned()),
                    at: Point { x: 40, y: 30 },
                    button: PointerButton::Secondary,
                })),
            ),
            (
                &[
                    "debug",
                    "pointer-release",
                    "sill-desktop-widgets-DP-1",
                    "0",
                    "456",
                ],
                Some(Request::Debug(DebugRequest::PointerRelease {
                    surface: SurfaceName("sill-desktop-widgets-DP-1".to_owned()),
                    at: Point { x: 0, y: 456 },
                    button: PointerButton::Primary,
                })),
            ),
            (
                &[
                    "debug",
                    "pointer-release",
                    "s",
                    "0",
                    "0",
                    "--button",
                    "middle",
                ],
                Some(Request::Debug(DebugRequest::PointerRelease {
                    surface: SurfaceName("s".to_owned()),
                    at: Point { x: 0, y: 0 },
                    button: PointerButton::Middle,
                })),
            ),
            (
                &["debug", "osd", "volume", "40"],
                Some(Request::Debug(DebugRequest::Osd(FakeOsd {
                    topic: OsdTopic::Volume,
                    percent: 40,
                    mute: FakeMute::Audible,
                }))),
            ),
            (
                &["debug", "osd", "volume", "40", "--mute", "muted"],
                Some(Request::Debug(DebugRequest::Osd(FakeOsd {
                    topic: OsdTopic::Volume,
                    percent: 40,
                    mute: FakeMute::Muted,
                }))),
            ),
            (
                &["debug", "osd", "brightness", "70"],
                Some(Request::Debug(DebugRequest::Osd(FakeOsd {
                    topic: OsdTopic::Brightness,
                    percent: 70,
                    mute: FakeMute::Audible,
                }))),
            ),
        ];
        for (args, want) in cases {
            let cli = Cli::try_parse_from(std::iter::once("sill").chain(args.iter().copied()))
                .unwrap_or_else(|e| panic!("{args:?}: {e}"));
            let command = cli
                .command
                .unwrap_or_else(|| panic!("{args:?}: no command"));
            assert_eq!(request(&command), want, "{args:?}");
        }
    }

    #[test]
    fn a_malformed_command_line_is_refused() {
        for args in [
            &["launcher"][..],
            &["power-menu"],
            &["control-center", "open"],
            &["dock", "pin"],
            &["debug", "pointer", "s", "x", "1"],
            &["debug", "click", "s", "1"],
            &["debug", "click", "s", "1", "2", "--button", "left"],
            &["debug", "pointer-press", "s", "1"],
            &["debug", "pointer-press", "s", "1", "2", "--button", "left"],
            &["debug", "pointer-release", "s", "1"],
            &[
                "debug",
                "pointer-release",
                "s",
                "1",
                "2",
                "--button",
                "left",
            ],
            &["debug", "osd", "sound", "40"],
            &["debug", "osd", "volume", "loud"],
            &["notifications"],
            &["notifications", "clear"],
            &["debug", "notify"],
            &["debug", "notify", "x", "--urgency", "urgent"],
            &["screenshot", "region"],
            &["widgets"],
            &["widgets", "toggle"],
            &["debug", "screenshot"],
            &["debug", "corner", "middle"],
            &["switcher"],
            &["switcher", "open"],
            &["lock", "unlock"],
            &["polkit"],
            &["debug", "auth"],
            &["debug", "auth", "maybe"],
            &["debug", "switcher"],
            &["frobnicate"],
        ] {
            assert!(
                Cli::try_parse_from(std::iter::once("sill").chain(args.iter().copied())).is_err(),
                "{args:?}"
            );
        }
    }

    #[test]
    fn write_schema_needs_no_subcommand() {
        let cli = Cli::try_parse_from(["sill", "--write-schema", "/tmp/schemas"])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(cli.write_schema, Some(PathBuf::from("/tmp/schemas")));
        assert!(cli.command.is_none());
    }
}
