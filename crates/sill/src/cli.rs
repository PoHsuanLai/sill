//! The command line (plan: `sill run|launcher {toggle,open [--query],close}|dock {pin,unpin}|
//! reload|quit|ping|debug {stats,pointer,leave,capture}`, plus `debug click` from the M2 freeze
//! amendment, `display {brightness,volume}` from the displays pass (`cli_display`), and
//! `power-menu {open,close,toggle}` and `control-center toggle` from the M5 freeze,
//! `notifications {toggle,open,close,dismiss-all,status}` and `debug notify` from the M6 freeze
//! (`cli_notifications`), and its one pure mapping to an IPC request.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub use crate::cli_display::{DisplayAction, LevelAction, VolumeAction, display_request};
pub use crate::cli_notifications::{
    ExpireArg, FakeNotifyArgs, NotificationsAction, UrgencyArg, fake_notification,
    notifications_request,
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
        Command::Run => return None,
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
    use std::path::PathBuf;

    #[test]
    fn every_command_line_sends_its_request() {
        let cases: Vec<(&[&str], Option<Request>)> = vec![
            (&["run"], None),
            (&["ping"], Some(Request::Ping)),
            (&["reload"], Some(Request::Reload)),
            (&["quit"], Some(Request::Quit)),
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
            &["debug", "osd", "sound", "40"],
            &["debug", "osd", "volume", "loud"],
            &["notifications"],
            &["notifications", "clear"],
            &["debug", "notify"],
            &["debug", "notify", "x", "--urgency", "urgent"],
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
