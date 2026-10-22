//! The command line's table: every accepted command line and the request it sends, and every
//! malformed one refused.

use std::path::PathBuf;

use clap::Parser;
use sill_ipc::act::{ActRequest, DisplayRequest, LevelRequest, VolumeRequest};
use sill_ipc::debug::{DebugRequest, KeyAction, KeySpec, Point, PointerButton, SurfaceName};
use sill_ipc::request::Request;
use sill_ipc::show::{CompanionRequest, LauncherRequest, Reveal, ShowRequest, VoiceRequest};
use sill_ipc::status::StatusRequest;
use sill_launcher::actions::ActionId as ShellAction;
use sill_launcher::settings_target::page;
use sill_launcher::{AppId, WindowKey};
use sill_model::actions::{Ask, Subject};
use sill_model::auth::FakeVerdict;
use sill_model::hot_corners::Corner;
use sill_model::intents::IntentsAsk;
use sill_model::intents::terminal::parse_grant;
use sill_model::keycap::KeycapAsk;
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
    parse_body,
};
use sill_model::osd::OsdKind;
use sill_model::polkit_agent::{ActionId, FakeRequest};
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

fn doing(action: ShellAction) -> Option<Request> {
    Some(Request::Do(Ask::of(action)))
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
        (&["next-window"], doing(ShellAction::NextWindow)),
        (&["dock", "toggle-hide"], doing(ShellAction::ToggleDockHide)),
        (
            &["launch", "org.mozilla.firefox"],
            act(ActRequest::Launch(AppId("org.mozilla.firefox".to_owned()))),
        ),
        (
            &["launch", "foot.desktop", "--print-timing"],
            act(ActRequest::Launch(AppId("foot".to_owned()))),
        ),
        (&["quit"], Some(Request::Quit)),
        (&["lock"], doing(ShellAction::Lock)),
        (&["lock", "now"], doing(ShellAction::Lock)),
        (&["lock", "status"], status(StatusRequest::Lock)),
        (&["switcher", "next"], doing(ShellAction::SwitcherNext)),
        (&["switcher", "prev"], doing(ShellAction::SwitcherPrev)),
        (&["switcher", "cancel"], doing(ShellAction::SwitcherCancel)),
        (&["switcher", "commit"], doing(ShellAction::SwitcherCommit)),
        (&["workspace", "next"], doing(ShellAction::NextSpace)),
        (&["workspace", "prev"], doing(ShellAction::PreviousSpace)),
        (
            &["workspace", "overview"],
            doing(ShellAction::MissionControl),
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
        (&["intents", "status"], status(StatusRequest::Intents)),
        (
            &["intents", "revoke", "org.quire.Shell:shell.dock.pin"],
            act(ActRequest::Intents(IntentsAsk::Revoke(
                parse_grant("org.quire.Shell:shell.dock.pin").expect("a grant"), // test-only
            ))),
        ),
        (
            &["intents", "resume"],
            act(ActRequest::Intents(IntentsAsk::Resume)),
        ),
        (&["clipboard", "status"], status(StatusRequest::Clipboard)),
        (&["clipboard", "clear"], act(ActRequest::ClearClipboard)),
        (&["screenshot"], doing(ShellAction::ScreenshotArea)),
        (
            &["screenshot", "window"],
            doing(ShellAction::ScreenshotWindow),
        ),
        (
            &["screenshot", "screen"],
            doing(ShellAction::ScreenshotScreen),
        ),
        (
            &["companion", "tap"],
            show(ShowRequest::Companion(CompanionRequest::Tap)),
        ),
        (
            &["companion", "summon"],
            show(ShowRequest::Companion(CompanionRequest::Summon)),
        ),
        (
            &["companion", "dismiss"],
            show(ShowRequest::Companion(CompanionRequest::Dismiss)),
        ),
        (
            &["companion", "stop-all"],
            show(ShowRequest::Companion(CompanionRequest::StopAll)),
        ),
        (
            &["voice", "hold-start"],
            show(ShowRequest::Voice(VoiceRequest::HoldStart)),
        ),
        (
            &["voice", "hold-end"],
            show(ShowRequest::Voice(VoiceRequest::HoldEnd)),
        ),
        (
            &["voice", "hold-cancel"],
            show(ShowRequest::Voice(VoiceRequest::HoldCancel)),
        ),
        (&["power-menu", "open"], doing(ShellAction::PowerMenu)),
        (
            &["power-menu", "close"],
            show(ShowRequest::PowerMenu(Reveal::Close)),
        ),
        (
            &["power-menu", "toggle"],
            show(ShowRequest::PowerMenu(Reveal::Toggle)),
        ),
        (
            &["force-quit", "open"],
            show(ShowRequest::ForceQuit(Reveal::Open)),
        ),
        (
            &["force-quit", "close"],
            show(ShowRequest::ForceQuit(Reveal::Close)),
        ),
        (
            &["force-quit", "toggle"],
            doing(ShellAction::ForceQuitPanel),
        ),
        (
            &["control-center", "toggle"],
            doing(ShellAction::ControlCenter),
        ),
        (
            &["notifications", "toggle"],
            doing(ShellAction::NotificationCenter),
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
        (&["launcher", "toggle"], doing(ShellAction::Launcher)),
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
            Some(Request::Do(Ask::on(
                ShellAction::PinApp,
                Subject::App(AppId("foot".to_owned())),
            ))),
        ),
        (
            &["dock", "unpin", "foot"],
            Some(Request::Do(Ask::on(
                ShellAction::UnpinApp,
                Subject::App(AppId("foot".to_owned())),
            ))),
        ),
        (&["do", "lock"], doing(ShellAction::Lock)),
        (
            &["do", "emoji-and-symbols"],
            doing(ShellAction::EmojiAndSymbols),
        ),
        (
            &["do", "next-input-source"],
            doing(ShellAction::NextInputSource),
        ),
        (
            &["do", "open-app", "--app", "foot"],
            Some(Request::Do(Ask::on(
                ShellAction::OpenApp,
                Subject::App(AppId("foot".to_owned())),
            ))),
        ),
        (
            &["do", "quit-app", "--windows", "4,7"],
            Some(Request::Do(Ask::on(
                ShellAction::QuitApp,
                Subject::Windows(vec![WindowKey(4), WindowKey(7)]),
            ))),
        ),
        (
            &["do", "open-settings", "--settings", "page:wallpaper"],
            Some(Request::Do(Ask::on(
                ShellAction::OpenSettings,
                Subject::Settings(page("wallpaper")),
            ))),
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
            &["debug", "mode", "s-1", "manual"],
            debug(DebugRequest::Mode {
                session: sill_model::companion::SessionId::parse("s-1")
                    .unwrap_or_else(|e| panic!("{e}")),
                mode: sill_model::companion::Mode::Manual,
            }),
        ),
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
            debug(DebugRequest::Corner(Corner::TopLeft)),
        ),
        (
            &["debug", "corner", "bottom-right"],
            debug(DebugRequest::Corner(Corner::BottomRight)),
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
            &["debug", "pointer-press", "sill-wallpaper-DP-1", "900", "96"],
            press("sill-wallpaper-DP-1", 900, 96, PointerButton::Primary),
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
                "sill-wallpaper-DP-1",
                "0",
                "456",
            ],
            release("sill-wallpaper-DP-1", 0, 456, PointerButton::Primary),
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
                topic: OsdKind::Volume,
                percent: Percent(40),
                mute: Mute::Audible,
            }),
        ),
        (
            &["debug", "osd", "volume", "40", "--mute", "muted"],
            debug(DebugRequest::Osd {
                topic: OsdKind::Volume,
                percent: Percent(40),
                mute: Mute::Muted,
            }),
        ),
        (
            &["debug", "osd", "brightness", "70"],
            debug(DebugRequest::Osd {
                topic: OsdKind::Brightness,
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
        &["force-quit"],
        &["control-center", "open"],
        &["dock", "pin"],
        &["do"],
        &["do", "reboot"],
        &["do", "open-app", "--app", "foot", "--windows", "1"],
        &["launch"],
        &["launch", "foot", "bar"],
        &["launch", "foot", "--timing"],
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

/// The verbs the compositor sends are read by their own parser and by clap's grammar, and both
/// come to the one request: the action `action_of` names. No string is copied here: the words are
/// the verb's own `argv()`.
#[test]
fn every_verb_the_compositor_sends_parses_and_reaches_its_action() {
    for verb in shell_verbs::ALL {
        let words = verb.argv();
        let want = Some(Request::Do(Ask::of(super::action_of(*verb))));
        assert_eq!(
            super::request_of_words(words),
            want,
            "{verb:?} by its words"
        );
        let line = std::iter::once("sill").chain(words.iter().copied());
        let by_clap = Cli::try_parse_from(line).ok().and_then(|cli| cli.command);
        assert_eq!(
            by_clap.as_ref().and_then(request),
            want,
            "{verb:?} by clap's grammar"
        );
    }
}

/// Every verb of the compositor reaches a different action.
#[test]
fn no_two_verbs_reach_one_action() {
    let actions: Vec<ShellAction> = shell_verbs::ALL
        .iter()
        .map(|verb| super::action_of(*verb))
        .collect();
    for (at, action) in actions.iter().enumerate() {
        assert!(
            !actions[..at].contains(action),
            "{action:?} is reached twice"
        );
    }
}
