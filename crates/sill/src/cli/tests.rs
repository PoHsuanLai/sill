//! The command line's table: every accepted command line and the request it sends, and every
//! malformed one refused.

use std::path::PathBuf;

use clap::Parser;
use sill_ipc::act::{
    ActRequest, DisplayRequest, DockRequest, LevelRequest, SwitcherRequest, VolumeRequest,
};
use sill_ipc::debug::{
    CornerName, DebugRequest, KeyAction, KeySpec, OsdTopic, Point, PointerButton, SurfaceName,
};
use sill_ipc::request::Request;
use sill_ipc::show::{EditWidgetsRequest, LauncherRequest, Reveal, ShowRequest};
use sill_ipc::status::StatusRequest;
use sill_launcher::AppId;
use sill_model::auth::FakeVerdict;
use sill_model::keycap::KeycapAsk;
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
    parse_body,
};
use sill_model::polkit_agent::{ActionId, FakeRequest};
use sill_model::screenshots::ShotKind;
use sill_model::units::Percent;
use sill_model::volume::Mute;

use super::{Cli, request};

fn surface(name: &str) -> SurfaceName {
    SurfaceName(name.to_owned())
}

fn debug(request: DebugRequest) -> Option<Request> {
    Some(Request::Debug(request))
}

fn show(request: ShowRequest) -> Option<Request> {
    Some(Request::Show(request))
}

fn act(request: ActRequest) -> Option<Request> {
    Some(Request::Act(request))
}

fn status(request: StatusRequest) -> Option<Request> {
    Some(Request::Status(request))
}

fn pointer(
    request: fn(SurfaceName, Point, PointerButton) -> DebugRequest,
) -> impl Fn(&str, i32, i32, PointerButton) -> Option<Request> {
    move |name, x, y, button| debug(request(surface(name), Point { x, y }, button))
}

fn click(surface: SurfaceName, at: Point, button: PointerButton) -> DebugRequest {
    DebugRequest::Click {
        surface,
        at,
        button,
    }
}

fn press(surface: SurfaceName, at: Point, button: PointerButton) -> DebugRequest {
    DebugRequest::PointerPress {
        surface,
        at,
        button,
    }
}

fn release(surface: SurfaceName, at: Point, button: PointerButton) -> DebugRequest {
    DebugRequest::PointerRelease {
        surface,
        at,
        button,
    }
}

/// A made-up notification as the daemon receives it.
fn notification(summary: &str) -> Posting {
    Posting {
        replaces: None,
        app: AppRef {
            name: "sill".to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: summary.to_owned(),
        body: parse_body(""),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

fn action_of(key: &str, label: &str) -> Action {
    Action {
        key: ActionKey(key.to_owned()),
        label: label.to_owned(),
    }
}

#[test]
fn every_command_line_sends_its_request() {
    let (click, press, release) = (pointer(click), pointer(press), pointer(release));
    let brightness = |l| act(ActRequest::Display(DisplayRequest::Brightness(l)));
    let volume = |v| act(ActRequest::Display(DisplayRequest::Volume(v)));
    let cases: Vec<(&[&str], Option<Request>)> = vec![
        (&["run"], None),
        (&["auth-check"], None),
        (&["style", "check"], None),
        (&["style", "check", "other.css"], None),
        (&["ping"], Some(Request::Ping)),
        (&["reload"], Some(Request::Reload)),
        (&["focus-latest"], act(ActRequest::FocusLatest)),
        (&["quit"], Some(Request::Quit)),
        (&["lock"], act(ActRequest::Lock)),
        (&["lock", "now"], act(ActRequest::Lock)),
        (&["lock", "status"], status(StatusRequest::Lock)),
        (
            &["switcher", "next"],
            act(ActRequest::Switcher(SwitcherRequest::Next)),
        ),
        (
            &["switcher", "prev"],
            act(ActRequest::Switcher(SwitcherRequest::Prev)),
        ),
        (
            &["switcher", "cancel"],
            act(ActRequest::Switcher(SwitcherRequest::Cancel)),
        ),
        (
            &["switcher", "commit"],
            act(ActRequest::Switcher(SwitcherRequest::Commit)),
        ),
        (&["polkit", "status"], status(StatusRequest::Polkit)),
        (&["idle", "status"], status(StatusRequest::Idle)),
        (&["keycap", "status"], status(StatusRequest::Keycap)),
        (
            &["keycap", "retry"],
            act(ActRequest::Keycap(KeycapAsk::Retry)),
        ),
        (
            &["keycap", "resume"],
            act(ActRequest::Keycap(KeycapAsk::Resume)),
        ),
        (&["clipboard", "status"], status(StatusRequest::Clipboard)),
        (&["clipboard", "clear"], act(ActRequest::ClearClipboard)),
        (&["screenshot"], act(ActRequest::Screenshot(ShotKind::Area))),
        (
            &["screenshot", "window"],
            act(ActRequest::Screenshot(ShotKind::Window)),
        ),
        (
            &["screenshot", "screen"],
            act(ActRequest::Screenshot(ShotKind::Screen)),
        ),
        (
            &["widgets", "toggle-desktop"],
            act(ActRequest::ToggleDesktopWidgets),
        ),
        (
            &["widgets", "edit"],
            show(ShowRequest::EditWidgets(EditWidgetsRequest::Open)),
        ),
        (
            &["widgets", "edit-close"],
            show(ShowRequest::EditWidgets(EditWidgetsRequest::Close)),
        ),
        (
            &["power-menu", "open"],
            show(ShowRequest::PowerMenu(Reveal::Open)),
        ),
        (
            &["power-menu", "close"],
            show(ShowRequest::PowerMenu(Reveal::Close)),
        ),
        (
            &["power-menu", "toggle"],
            show(ShowRequest::PowerMenu(Reveal::Toggle)),
        ),
        (
            &["control-center", "toggle"],
            show(ShowRequest::ControlCenter),
        ),
        (
            &["notifications", "toggle"],
            show(ShowRequest::NotificationCenter(Reveal::Toggle)),
        ),
        (
            &["notifications", "open"],
            show(ShowRequest::NotificationCenter(Reveal::Open)),
        ),
        (
            &["notifications", "close"],
            show(ShowRequest::NotificationCenter(Reveal::Close)),
        ),
        (
            &["notifications", "dismiss-all"],
            act(ActRequest::DismissNotifications),
        ),
        (
            &["notifications", "status"],
            status(StatusRequest::Notifications),
        ),
        (
            &["launcher", "toggle"],
            show(ShowRequest::Launcher(LauncherRequest::Toggle)),
        ),
        (
            &["launcher", "close"],
            show(ShowRequest::Launcher(LauncherRequest::Close)),
        ),
        (
            &["launcher", "open"],
            show(ShowRequest::Launcher(LauncherRequest::Open {
                query: String::new(),
            })),
        ),
        (
            &["launcher", "open", "--query", ""],
            show(ShowRequest::Launcher(LauncherRequest::Open {
                query: String::new(),
            })),
        ),
        (
            &["launcher", "open", "--query", "fire"],
            show(ShowRequest::Launcher(LauncherRequest::Open {
                query: "fire".to_owned(),
            })),
        ),
        (
            &["dock", "pin", "foot"],
            act(ActRequest::Dock(DockRequest::Pin {
                app: AppId("foot".to_owned()),
            })),
        ),
        (
            &["dock", "unpin", "foot"],
            act(ActRequest::Dock(DockRequest::Unpin {
                app: AppId("foot".to_owned()),
            })),
        ),
        (
            &["display", "brightness", "up"],
            brightness(LevelRequest::Up),
        ),
        (
            &["display", "brightness", "set", "40"],
            brightness(LevelRequest::Set { percent: 40 }),
        ),
        (
            &["display", "volume", "down"],
            volume(VolumeRequest::Level(LevelRequest::Down)),
        ),
        (&["display", "volume", "mute"], volume(VolumeRequest::Mute)),
        (
            &["display", "volume", "toggle-mute"],
            volume(VolumeRequest::ToggleMute),
        ),
        (&["debug", "stats"], debug(DebugRequest::Stats)),
        (
            &["debug", "launcher-rows"],
            debug(DebugRequest::LauncherRows),
        ),
        (&["debug", "idle-wake"], debug(DebugRequest::IdleWake)),
        (
            &["debug", "auth", "accept"],
            debug(DebugRequest::Auth(FakeVerdict::Accept)),
        ),
        (
            &["debug", "auth", "refuse"],
            debug(DebugRequest::Auth(FakeVerdict::Refuse)),
        ),
        (
            &["debug", "polkit"],
            debug(DebugRequest::Polkit(FakeRequest {
                action_id: ActionId("org.sill.debug.prompt".to_owned()),
                message: "sill is testing its password prompt".to_owned(),
                icon: String::new(),
            })),
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
            debug(DebugRequest::Polkit(FakeRequest {
                action_id: ActionId("org.example.run".to_owned()),
                message: "Run?".to_owned(),
                icon: String::new(),
            })),
        ),
        (
            &["debug", "switcher", "foot", "firefox"],
            debug(DebugRequest::Switcher {
                apps: vec![AppId("foot".to_owned()), AppId("firefox".to_owned())],
            }),
        ),
        (
            &["debug", "screenshot", "/tmp/shots/Screenshot_1.png"],
            debug(DebugRequest::Screenshot {
                path: PathBuf::from("/tmp/shots/Screenshot_1.png"),
            }),
        ),
        (
            &["debug", "corner", "top-left"],
            debug(DebugRequest::Corner(CornerName::TopLeft)),
        ),
        (
            &["debug", "corner", "bottom-right"],
            debug(DebugRequest::Corner(CornerName::BottomRight)),
        ),
        (
            &["debug", "notify", "Hello"],
            debug(DebugRequest::Notify(Box::new(notification("Hello")))),
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
            debug(DebugRequest::Notify(Box::new(Posting {
                replaces: Some(NotificationId(4)),
                app: AppRef {
                    name: "Mail".to_owned(),
                    icon: IconRef::None,
                    desktop_entry: None,
                },
                summary: "Ada".to_owned(),
                body: parse_body("the <b>engine</b>"),
                actions: vec![action_of("default", "Open"), action_of("reply", "Reply")],
                hints: Hints {
                    urgency: NotifyUrgency::Critical,
                    ..Hints::default()
                },
                expiry: Expiry::Never,
            }))),
        ),
        (
            &["debug", "pointer", "sill-dock-DP-1", "300", "64"],
            debug(DebugRequest::Pointer {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 300, y: 64 },
            }),
        ),
        (
            &["debug", "leave", "sill-dock-DP-1"],
            debug(DebugRequest::Leave {
                surface: surface("sill-dock-DP-1"),
            }),
        ),
        (
            &["debug", "capture", "sill-bar-DP-1", "/tmp/bar.png"],
            debug(DebugRequest::Capture {
                surface: surface("sill-bar-DP-1"),
                path: PathBuf::from("/tmp/bar.png"),
            }),
        ),
        (
            &["debug", "key", "sill-power-menu-DP-1", "ArrowRight"],
            debug(DebugRequest::Key {
                surface: surface("sill-power-menu-DP-1"),
                key: KeySpec("ArrowRight".to_owned()),
                action: KeyAction::Tap,
            }),
        ),
        (
            &[
                "debug",
                "key",
                "sill-launcher-DP-1",
                "k+ctrl",
                "--action",
                "press",
            ],
            debug(DebugRequest::Key {
                surface: surface("sill-launcher-DP-1"),
                key: KeySpec("k+ctrl".to_owned()),
                action: KeyAction::Press,
            }),
        ),
        (
            &["debug", "click", "sill-bar-DP-1", "1200", "16"],
            click("sill-bar-DP-1", 1200, 16, PointerButton::Primary),
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
            click("sill-dock-DP-1", 40, 30, PointerButton::Secondary),
        ),
        (
            &["debug", "click", "s", "0", "0", "--button", "middle"],
            click("s", 0, 0, PointerButton::Middle),
        ),
        (
            &[
                "debug",
                "pointer-press",
                "sill-desktop-widgets-DP-1",
                "900",
                "96",
            ],
            press("sill-desktop-widgets-DP-1", 900, 96, PointerButton::Primary),
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
            press("sill-dock-DP-1", 40, 30, PointerButton::Secondary),
        ),
        (
            &[
                "debug",
                "pointer-release",
                "sill-desktop-widgets-DP-1",
                "0",
                "456",
            ],
            release("sill-desktop-widgets-DP-1", 0, 456, PointerButton::Primary),
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
            release("s", 0, 0, PointerButton::Middle),
        ),
        (
            &["debug", "osd", "volume", "40"],
            debug(DebugRequest::Osd {
                topic: OsdTopic::Volume,
                percent: Percent(40),
                mute: Mute::Audible,
            }),
        ),
        (
            &["debug", "osd", "volume", "40", "--mute", "muted"],
            debug(DebugRequest::Osd {
                topic: OsdTopic::Volume,
                percent: Percent(40),
                mute: Mute::Muted,
            }),
        ),
        (
            &["debug", "osd", "brightness", "70"],
            debug(DebugRequest::Osd {
                topic: OsdTopic::Brightness,
                percent: Percent(70),
                mute: Mute::Audible,
            }),
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
