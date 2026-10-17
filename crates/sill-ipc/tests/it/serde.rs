//! Every request and response round-trips through one line, the wire spelling of each variant
//! is pinned, and a malformed line is refused rather than guessed at.

use std::io::Cursor;
use std::path::PathBuf;

use keycap_protocol::{Grab, Keyboard, KeyboardInfo, Mode, Status};
use sill_ipc::act::{
    ActRequest, DisplayRequest, DockRequest, LevelRequest, SwitcherRequest, VolumeRequest,
    WorkspaceRequest,
};
use sill_ipc::debug::{
    CornerName, DebugRequest, KeyAction, KeySpec, LauncherRows, Micros, OsdTopic, Point,
    PointerButton, RowsSection, SurfaceName, SurfaceStats,
};
use sill_ipc::error::IpcError;
use sill_ipc::line::{MAX_LINE, decode, encode, read_message, write_message};
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_ipc::show::{
    CompanionRequest, EditWidgetsRequest, LauncherRequest, Reveal, ShowRequest, VoiceRequest,
};
use sill_ipc::status::{
    ClipboardStatus, IntentsStatus, LinkStatus, NotificationsStatus, PolkitStatus, StatusReport,
    StatusRequest,
};
use sill_launcher::{AppId, PreviewKind};
use sill_model::auth::FakeVerdict;
use sill_model::clipboard::{ClipProtocol, ClipStatus, HistoryMode};
use sill_model::idle::{
    DimLevel, DimShown, Idle, IdleMode, IdlePhase, Inhibition, LogindIdle, OffReason, PowerSource,
};
use sill_model::intents::IntentsAsk;
use sill_model::intents::terminal::{Breaker, ControlReach, parse_grant};
use sill_model::keycap::{KeycapAsk, KeycapState};
use sill_model::launch::{LaunchTiming, Pid, ScopeUnit, Stamp};
use sill_model::locker::{Lock, LockCause, LockPhase};
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, NameHolder, NameOwner, NotifyUrgency,
    Posting, ServerMode, ServerStatus, parse_body,
};
use sill_model::polkit_agent::{ActionId, FakeRequest, Registration};
use sill_model::screenshots::ShotKind;
use sill_model::units::Percent;
use sill_model::volume::Mute;

fn surface(name: &str) -> SurfaceName {
    SurfaceName(name.to_owned())
}

fn app(id: &str) -> AppId {
    AppId(id.to_owned())
}

fn notification() -> Posting {
    Posting {
        replaces: None,
        app: AppRef {
            name: "Mail".to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: "Ada".to_owned(),
        body: parse_body("the <b>engine</b>"),
        actions: vec![Action {
            key: ActionKey("default".to_owned()),
            label: "Open".to_owned(),
        }],
        hints: Hints {
            urgency: NotifyUrgency::Critical,
            ..Hints::default()
        },
        expiry: Expiry::Never,
    }
}

/// Every request variant, with the exact line it is written as.
fn requests() -> Vec<(Request, &'static str)> {
    vec![
        (Request::Ping, r#"{"kind":"ping"}"#),
        (Request::Quit, r#"{"kind":"quit"}"#),
        (Request::Reload, r#"{"kind":"reload"}"#),
        (
            Request::Show(ShowRequest::Launcher(LauncherRequest::Toggle)),
            r#"{"kind":"show","v":{"kind":"launcher","v":{"kind":"toggle"}}}"#,
        ),
        (
            Request::Show(ShowRequest::Launcher(LauncherRequest::Open {
                query: "fire".to_owned(),
            })),
            r#"{"kind":"show","v":{"kind":"launcher","v":{"kind":"open","v":{"query":"fire"}}}}"#,
        ),
        (
            Request::Show(ShowRequest::Launcher(LauncherRequest::Open {
                query: String::new(),
            })),
            r#"{"kind":"show","v":{"kind":"launcher","v":{"kind":"open","v":{"query":""}}}}"#,
        ),
        (
            Request::Show(ShowRequest::Launcher(LauncherRequest::Close)),
            r#"{"kind":"show","v":{"kind":"launcher","v":{"kind":"close"}}}"#,
        ),
        (
            Request::Show(ShowRequest::Companion(CompanionRequest::Tap)),
            r#"{"kind":"show","v":{"kind":"companion","v":"tap"}}"#,
        ),
        (
            Request::Show(ShowRequest::Companion(CompanionRequest::Summon)),
            r#"{"kind":"show","v":{"kind":"companion","v":"summon"}}"#,
        ),
        (
            Request::Show(ShowRequest::Companion(CompanionRequest::Dismiss)),
            r#"{"kind":"show","v":{"kind":"companion","v":"dismiss"}}"#,
        ),
        (
            Request::Show(ShowRequest::Companion(CompanionRequest::StopAll)),
            r#"{"kind":"show","v":{"kind":"companion","v":"stop_all"}}"#,
        ),
        (
            Request::Show(ShowRequest::Voice(VoiceRequest::HoldStart)),
            r#"{"kind":"show","v":{"kind":"voice","v":"hold_start"}}"#,
        ),
        (
            Request::Show(ShowRequest::Voice(VoiceRequest::HoldEnd)),
            r#"{"kind":"show","v":{"kind":"voice","v":"hold_end"}}"#,
        ),
        (
            Request::Show(ShowRequest::Voice(VoiceRequest::HoldCancel)),
            r#"{"kind":"show","v":{"kind":"voice","v":"hold_cancel"}}"#,
        ),
        (
            Request::Show(ShowRequest::PowerMenu(Reveal::Open)),
            r#"{"kind":"show","v":{"kind":"power_menu","v":"open"}}"#,
        ),
        (
            Request::Show(ShowRequest::PowerMenu(Reveal::Close)),
            r#"{"kind":"show","v":{"kind":"power_menu","v":"close"}}"#,
        ),
        (
            Request::Show(ShowRequest::PowerMenu(Reveal::Toggle)),
            r#"{"kind":"show","v":{"kind":"power_menu","v":"toggle"}}"#,
        ),
        (
            Request::Show(ShowRequest::ForceQuit(Reveal::Toggle)),
            r#"{"kind":"show","v":{"kind":"force_quit","v":"toggle"}}"#,
        ),
        (
            Request::Show(ShowRequest::NotificationCenter(Reveal::Toggle)),
            r#"{"kind":"show","v":{"kind":"notification_center","v":"toggle"}}"#,
        ),
        (
            Request::Show(ShowRequest::NotificationCenter(Reveal::Open)),
            r#"{"kind":"show","v":{"kind":"notification_center","v":"open"}}"#,
        ),
        (
            Request::Show(ShowRequest::NotificationCenter(Reveal::Close)),
            r#"{"kind":"show","v":{"kind":"notification_center","v":"close"}}"#,
        ),
        (
            Request::Show(ShowRequest::ControlCenter),
            r#"{"kind":"show","v":{"kind":"control_center"}}"#,
        ),
        (
            Request::Show(ShowRequest::EditWidgets(EditWidgetsRequest::Open)),
            r#"{"kind":"show","v":{"kind":"edit_widgets","v":"open"}}"#,
        ),
        (
            Request::Show(ShowRequest::EditWidgets(EditWidgetsRequest::Close)),
            r#"{"kind":"show","v":{"kind":"edit_widgets","v":"close"}}"#,
        ),
        (
            Request::Status(StatusRequest::Notifications),
            r#"{"kind":"status","v":"notifications"}"#,
        ),
        (
            Request::Status(StatusRequest::Lock),
            r#"{"kind":"status","v":"lock"}"#,
        ),
        (
            Request::Status(StatusRequest::Polkit),
            r#"{"kind":"status","v":"polkit"}"#,
        ),
        (
            Request::Status(StatusRequest::Clipboard),
            r#"{"kind":"status","v":"clipboard"}"#,
        ),
        (
            Request::Status(StatusRequest::Idle),
            r#"{"kind":"status","v":"idle"}"#,
        ),
        (
            Request::Status(StatusRequest::Keycap),
            r#"{"kind":"status","v":"keycap"}"#,
        ),
        (
            Request::Status(StatusRequest::Intents),
            r#"{"kind":"status","v":"intents"}"#,
        ),
        (
            Request::Act(ActRequest::Intents(IntentsAsk::Revoke(
                parse_grant("org.quire.Shell:shell.dock.pin").expect("a grant"), // test-only
            ))),
            r#"{"kind":"act","v":{"kind":"intents","v":{"kind":"revoke","v":{"app":"org.quire.Shell","name":"shell.dock.pin"}}}}"#,
        ),
        (
            Request::Act(ActRequest::Intents(IntentsAsk::Resume)),
            r#"{"kind":"act","v":{"kind":"intents","v":{"kind":"resume"}}}"#,
        ),
        (
            Request::Act(ActRequest::Dock(DockRequest::Pin {
                app: app("org.mozilla.firefox"),
            })),
            r#"{"kind":"act","v":{"kind":"dock","v":{"kind":"pin","v":{"app":"org.mozilla.firefox"}}}}"#,
        ),
        (
            Request::Act(ActRequest::Dock(DockRequest::Unpin { app: app("foot") })),
            r#"{"kind":"act","v":{"kind":"dock","v":{"kind":"unpin","v":{"app":"foot"}}}}"#,
        ),
        (
            Request::Act(ActRequest::Display(DisplayRequest::Brightness(
                LevelRequest::Up,
            ))),
            r#"{"kind":"act","v":{"kind":"display","v":{"kind":"brightness","v":{"kind":"up"}}}}"#,
        ),
        (
            Request::Act(ActRequest::Display(DisplayRequest::Brightness(
                LevelRequest::Set { percent: 40 },
            ))),
            r#"{"kind":"act","v":{"kind":"display","v":{"kind":"brightness","v":{"kind":"set","v":{"percent":40}}}}}"#,
        ),
        (
            Request::Act(ActRequest::Display(DisplayRequest::Volume(
                VolumeRequest::Level(LevelRequest::Down),
            ))),
            r#"{"kind":"act","v":{"kind":"display","v":{"kind":"volume","v":{"kind":"level","v":{"kind":"down"}}}}}"#,
        ),
        (
            Request::Act(ActRequest::Display(DisplayRequest::Volume(
                VolumeRequest::ToggleMute,
            ))),
            r#"{"kind":"act","v":{"kind":"display","v":{"kind":"volume","v":{"kind":"toggle_mute"}}}}"#,
        ),
        (
            Request::Act(ActRequest::Lock),
            r#"{"kind":"act","v":{"kind":"lock"}}"#,
        ),
        (
            Request::Act(ActRequest::Screenshot(ShotKind::Area)),
            r#"{"kind":"act","v":{"kind":"screenshot","v":"area"}}"#,
        ),
        (
            Request::Act(ActRequest::Screenshot(ShotKind::Window)),
            r#"{"kind":"act","v":{"kind":"screenshot","v":"window"}}"#,
        ),
        (
            Request::Act(ActRequest::Screenshot(ShotKind::Screen)),
            r#"{"kind":"act","v":{"kind":"screenshot","v":"screen"}}"#,
        ),
        (
            Request::Act(ActRequest::ToggleDesktopWidgets),
            r#"{"kind":"act","v":{"kind":"toggle_desktop_widgets"}}"#,
        ),
        (
            Request::Act(ActRequest::Switcher(SwitcherRequest::Next)),
            r#"{"kind":"act","v":{"kind":"switcher","v":"next"}}"#,
        ),
        (
            Request::Act(ActRequest::Switcher(SwitcherRequest::Prev)),
            r#"{"kind":"act","v":{"kind":"switcher","v":"prev"}}"#,
        ),
        (
            Request::Act(ActRequest::Switcher(SwitcherRequest::Cancel)),
            r#"{"kind":"act","v":{"kind":"switcher","v":"cancel"}}"#,
        ),
        (
            Request::Act(ActRequest::Workspace(WorkspaceRequest::Next)),
            r#"{"kind":"act","v":{"kind":"workspace","v":"next"}}"#,
        ),
        (
            Request::Act(ActRequest::Workspace(WorkspaceRequest::Prev)),
            r#"{"kind":"act","v":{"kind":"workspace","v":"prev"}}"#,
        ),
        (
            Request::Act(ActRequest::Workspace(WorkspaceRequest::Overview)),
            r#"{"kind":"act","v":{"kind":"workspace","v":"overview"}}"#,
        ),
        (
            Request::Act(ActRequest::Switcher(SwitcherRequest::Commit)),
            r#"{"kind":"act","v":{"kind":"switcher","v":"commit"}}"#,
        ),
        (
            Request::Act(ActRequest::ClearClipboard),
            r#"{"kind":"act","v":{"kind":"clear_clipboard"}}"#,
        ),
        (
            Request::Act(ActRequest::DismissNotifications),
            r#"{"kind":"act","v":{"kind":"dismiss_notifications"}}"#,
        ),
        (
            Request::Act(ActRequest::FocusLatest),
            r#"{"kind":"act","v":{"kind":"focus_latest"}}"#,
        ),
        (
            Request::Act(ActRequest::Launch(app("org.mozilla.firefox"))),
            r#"{"kind":"act","v":{"kind":"launch","v":"org.mozilla.firefox"}}"#,
        ),
        (
            Request::Act(ActRequest::Keycap(KeycapAsk::Retry)),
            r#"{"kind":"act","v":{"kind":"keycap","v":"retry"}}"#,
        ),
        (
            Request::Act(ActRequest::Keycap(KeycapAsk::Resume)),
            r#"{"kind":"act","v":{"kind":"keycap","v":"resume"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Stats),
            r#"{"kind":"debug","v":{"kind":"stats"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Pointer {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 300, y: -1 },
            }),
            r#"{"kind":"debug","v":{"kind":"pointer","v":{"surface":"sill-dock-DP-1","at":{"x":300,"y":-1}}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Leave {
                surface: surface("sill-dock-DP-1"),
            }),
            r#"{"kind":"debug","v":{"kind":"leave","v":{"surface":"sill-dock-DP-1"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Capture {
                surface: surface("sill-bar-DP-1"),
                path: PathBuf::from("/tmp/bar.png"),
            }),
            r#"{"kind":"debug","v":{"kind":"capture","v":{"surface":"sill-bar-DP-1","path":"/tmp/bar.png"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Click {
                surface: surface("sill-bar-DP-1"),
                at: Point { x: 1200, y: 16 },
                button: PointerButton::Primary,
            }),
            r#"{"kind":"debug","v":{"kind":"click","v":{"surface":"sill-bar-DP-1","at":{"x":1200,"y":16},"button":"primary"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Click {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 40, y: 30 },
                button: PointerButton::Secondary,
            }),
            r#"{"kind":"debug","v":{"kind":"click","v":{"surface":"sill-dock-DP-1","at":{"x":40,"y":30},"button":"secondary"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Click {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 0, y: 0 },
                button: PointerButton::Middle,
            }),
            r#"{"kind":"debug","v":{"kind":"click","v":{"surface":"sill-dock-DP-1","at":{"x":0,"y":0},"button":"middle"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Key {
                surface: surface("sill-power-menu-DP-1"),
                key: KeySpec("ArrowRight".to_owned()),
                action: KeyAction::Tap,
            }),
            r#"{"kind":"debug","v":{"kind":"key","v":{"surface":"sill-power-menu-DP-1","key":"ArrowRight","action":"tap"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Key {
                surface: surface("sill-launcher-DP-1"),
                key: KeySpec("k+ctrl".to_owned()),
                action: KeyAction::Press,
            }),
            r#"{"kind":"debug","v":{"kind":"key","v":{"surface":"sill-launcher-DP-1","key":"k+ctrl","action":"press"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Key {
                surface: surface("sill-launcher-DP-1"),
                key: KeySpec("Tab+shift".to_owned()),
                action: KeyAction::Release,
            }),
            r#"{"kind":"debug","v":{"kind":"key","v":{"surface":"sill-launcher-DP-1","key":"Tab+shift","action":"release"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::PointerPress {
                surface: surface("sill-desktop-widgets-DP-1"),
                at: Point { x: 900, y: 96 },
                button: PointerButton::Primary,
            }),
            r#"{"kind":"debug","v":{"kind":"pointer_press","v":{"surface":"sill-desktop-widgets-DP-1","at":{"x":900,"y":96},"button":"primary"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::PointerPress {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 40, y: 30 },
                button: PointerButton::Secondary,
            }),
            r#"{"kind":"debug","v":{"kind":"pointer_press","v":{"surface":"sill-dock-DP-1","at":{"x":40,"y":30},"button":"secondary"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::PointerRelease {
                surface: surface("sill-desktop-widgets-DP-1"),
                at: Point { x: 0, y: 456 },
                button: PointerButton::Primary,
            }),
            r#"{"kind":"debug","v":{"kind":"pointer_release","v":{"surface":"sill-desktop-widgets-DP-1","at":{"x":0,"y":456},"button":"primary"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::PointerRelease {
                surface: surface("sill-dock-DP-1"),
                at: Point { x: 0, y: 0 },
                button: PointerButton::Middle,
            }),
            r#"{"kind":"debug","v":{"kind":"pointer_release","v":{"surface":"sill-dock-DP-1","at":{"x":0,"y":0},"button":"middle"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Osd {
                topic: OsdTopic::Volume,
                percent: Percent(40),
                mute: Mute::Muted,
            }),
            r#"{"kind":"debug","v":{"kind":"osd","v":{"topic":"volume","percent":40,"mute":"muted"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Notify(Box::new(notification()))),
            r#"{"kind":"debug","v":{"kind":"notify","v":{"replaces":null,"app":{"name":"Mail","icon":{"kind":"none"},"desktop_entry":null},"summary":"Ada","body":{"runs":[{"text":"the ","style":{"weight":"regular","slant":"upright","underline":"none","link":{"kind":"plain"}}},{"text":"engine","style":{"weight":"bold","slant":"upright","underline":"none","link":{"kind":"plain"}}}]},"actions":[{"key":"default","label":"Open"}],"hints":{"urgency":"critical","category":null,"image":{"kind":"none"},"transience":"kept","residency":"closes_on_action","sound":{"kind":"default"}},"expiry":{"kind":"never"}}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Screenshot {
                path: PathBuf::from("/tmp/shots/Screenshot_1.png"),
            }),
            r#"{"kind":"debug","v":{"kind":"screenshot","v":{"path":"/tmp/shots/Screenshot_1.png"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Corner(CornerName::BottomRight)),
            r#"{"kind":"debug","v":{"kind":"corner","v":"bottom_right"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Auth(FakeVerdict::Accept)),
            r#"{"kind":"debug","v":{"kind":"auth","v":"accept"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Auth(FakeVerdict::Refuse)),
            r#"{"kind":"debug","v":{"kind":"auth","v":"refuse"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Polkit(FakeRequest {
                action_id: ActionId("org.example.run".to_owned()),
                message: "Run it".to_owned(),
                icon: String::new(),
            })),
            r#"{"kind":"debug","v":{"kind":"polkit","v":{"action_id":"org.example.run","message":"Run it","icon":""}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Switcher {
                apps: vec![app("foot"), app("firefox")],
            }),
            r#"{"kind":"debug","v":{"kind":"switcher","v":{"apps":["foot","firefox"]}}}"#,
        ),
        (
            Request::Debug(DebugRequest::LauncherRows),
            r#"{"kind":"debug","v":{"kind":"launcher_rows"}}"#,
        ),
        (
            Request::Debug(DebugRequest::IdleWake),
            r#"{"kind":"debug","v":{"kind":"idle_wake"}}"#,
        ),
    ]
}

fn stats() -> SurfaceStats {
    SurfaceStats {
        surface: surface("sill-bar-DP-1"),
        frames: 12,
        callbacks: 12,
        commits: 13,
        skipped: 0,
        watchdog_fired: 0,
        work: Micros {
            p50_us: 900,
            p95_us: 2100,
            max_us: 4000,
        },
        interval: Micros {
            p50_us: 16_600,
            p95_us: 16_900,
            max_us: 33_000,
        },
        samples: 12,
        vm_rss_kib: 81_234,
    }
}

fn responses() -> Vec<(Response, &'static str)> {
    vec![
        (Response::Pong, r#"{"kind":"pong"}"#),
        (Response::Done, r#"{"kind":"done"}"#),
        (Response::Stats(Vec::new()), r#"{"kind":"stats","v":[]}"#),
        (
            Response::Launched(LaunchTiming {
                app: app("foot"),
                unit: Some(ScopeUnit("app-sill-foot-1.scope".to_owned())),
                requested_at: Stamp(100),
                spawned_at: Stamp(18_446_744_073_709_551_615),
                pid: Some(Pid(4242)),
            }),
            r#"{"kind":"launched","v":{"app":"foot","unit":"app-sill-foot-1.scope","requested_at":100,"spawned_at":18446744073709551615,"pid":4242}}"#,
        ),
        (
            Response::Launched(LaunchTiming {
                app: app("org.gnome.Calculator"),
                unit: None,
                requested_at: Stamp(7),
                spawned_at: Stamp(9),
                pid: None,
            }),
            r#"{"kind":"launched","v":{"app":"org.gnome.Calculator","unit":null,"requested_at":7,"spawned_at":9,"pid":null}}"#,
        ),
        (
            Response::Failed(Failure {
                kind: FailureKind::UnknownApp,
                message: "no app has that id".to_owned(),
            }),
            r#"{"kind":"failed","v":{"kind":"unknown_app","message":"no app has that id"}}"#,
        ),
        (
            Response::Failed(Failure {
                kind: FailureKind::UnknownSurface,
                message: "no surface sill-dock-HDMI-9".to_owned(),
            }),
            r#"{"kind":"failed","v":{"kind":"unknown_surface","message":"no surface sill-dock-HDMI-9"}}"#,
        ),
        (
            Response::Status(StatusReport::Notifications(NotificationsStatus {
                server: ServerStatus {
                    mode: ServerMode::Own,
                    holder: NameHolder::Other(NameOwner {
                        unique: ":1.42".to_owned(),
                        pid: Some(7),
                        command: Some("plasmashell".to_owned()),
                    }),
                },
                banners: 2,
                kept: 9,
            })),
            r#"{"kind":"status","v":{"kind":"notifications","v":{"server":{"mode":"own","holder":{"kind":"other","v":{"unique":":1.42","pid":7,"command":"plasmashell"}}},"banners":2,"kept":9}}}"#,
        ),
        (
            Response::Status(StatusReport::Notifications(NotificationsStatus {
                server: ServerStatus {
                    mode: ServerMode::Passive,
                    holder: NameHolder::Sill,
                },
                banners: 0,
                kept: 0,
            })),
            r#"{"kind":"status","v":{"kind":"notifications","v":{"server":{"mode":"passive","holder":{"kind":"sill"}},"banners":0,"kept":0}}}"#,
        ),
        (
            Response::Status(StatusReport::Lock(Lock {
                phase: LockPhase::Locked,
                cause: Some(LockCause::IdleScreenOff),
            })),
            r#"{"kind":"status","v":{"kind":"lock","v":{"phase":"locked","cause":"idle_screen_off"}}}"#,
        ),
        (
            Response::Status(StatusReport::Lock(Lock {
                phase: LockPhase::Unlocked,
                cause: None,
            })),
            r#"{"kind":"status","v":{"kind":"lock","v":{"phase":"unlocked","cause":null}}}"#,
        ),
        (
            Response::Status(StatusReport::Intents(IntentsStatus {
                link: LinkStatus::Serving,
                showing: Some("c-1".to_owned()),
                waiting: 2,
                grants: vec!["org.quire.Shell:shell.dock.pin".to_owned()],
                breaker: Breaker::Tripped,
                reach: ControlReach::Reached,
            })),
            r#"{"kind":"status","v":{"kind":"intents","v":{"link":"serving","showing":"c-1","waiting":2,"grants":["org.quire.Shell:shell.dock.pin"],"breaker":"tripped","reach":"reached"}}}"#,
        ),
        (
            Response::Status(StatusReport::Keycap(KeycapState::Failed)),
            r#"{"kind":"status","v":{"kind":"keycap","v":{"kind":"failed"}}}"#,
        ),
        (
            Response::Status(StatusReport::Keycap(KeycapState::Connected {
                status: Status {
                    mode: Mode::Paused,
                    keyboards: vec![Keyboard {
                        info: KeyboardInfo {
                            name: "Apple Keyboard".to_owned(),
                            path: "/dev/input/event4".to_owned(),
                            vendor: 0x5ac,
                            product: 0x250,
                        },
                        grab: Grab::Conflict,
                    }],
                },
            })),
            r#"{"kind":"status","v":{"kind":"keycap","v":{"kind":"connected","v":{"status":{"mode":"paused","keyboards":[{"info":{"name":"Apple Keyboard","path":"/dev/input/event4","vendor":1452,"product":592},"grab":"conflict"}]}}}}}"#,
        ),
        (
            Response::Status(StatusReport::Idle(Idle {
                mode: IdleMode::Own,
                phase: IdlePhase::DisplaysOff,
                dim: DimShown::Clear,
                power: PowerSource::Battery,
                inhibition: Inhibition::Held,
                inhibits: 1,
                logind: LogindIdle::Free,
            })),
            r#"{"kind":"status","v":{"kind":"idle","v":{"mode":{"kind":"own"},"phase":"displays_off","dim":{"kind":"clear"},"power":"battery","inhibition":"held","inhibits":1,"logind":"free"}}}"#,
        ),
        (
            Response::Status(StatusReport::Idle(Idle {
                mode: IdleMode::Off(OffReason::Guest),
                phase: IdlePhase::Dimmed,
                dim: DimShown::Dimmed(DimLevel(50)),
                power: PowerSource::Ac,
                inhibition: Inhibition::Free,
                inhibits: 0,
                logind: LogindIdle::Blocked,
            })),
            r#"{"kind":"status","v":{"kind":"idle","v":{"mode":{"kind":"off","v":"guest"},"phase":"dimmed","dim":{"kind":"dimmed","v":50},"power":"ac","inhibition":"free","inhibits":0,"logind":"blocked"}}}"#,
        ),
        (
            Response::Status(StatusReport::Polkit(PolkitStatus {
                registration: Registration::Refused("taken".to_owned()),
                showing: Some(ActionId("org.example.run".to_owned())),
                waiting: 1,
            })),
            r#"{"kind":"status","v":{"kind":"polkit","v":{"registration":{"kind":"refused","v":"taken"},"showing":"org.example.run","waiting":1}}}"#,
        ),
        (
            Response::Status(StatusReport::Polkit(PolkitStatus {
                registration: Registration::Passive,
                showing: None,
                waiting: 0,
            })),
            r#"{"kind":"status","v":{"kind":"polkit","v":{"registration":{"kind":"passive"},"showing":null,"waiting":0}}}"#,
        ),
        (
            Response::Status(StatusReport::Clipboard(ClipboardStatus {
                status: ClipStatus {
                    mode: HistoryMode::Memory,
                    protocol: ClipProtocol::Ext,
                },
                entries: 3,
            })),
            r#"{"kind":"status","v":{"kind":"clipboard","v":{"status":{"mode":"memory","protocol":"ext"},"entries":3}}}"#,
        ),
        (
            Response::LauncherRows(LauncherRows {
                query: "rep".to_owned(),
                sections: vec![RowsSection {
                    title: "Documents".to_owned(),
                    rows: vec!["report.pdf".to_owned()],
                    hidden: 2,
                }],
                selected: 0,
                preview: Some(PreviewKind::File),
            }),
            r#"{"kind":"launcher_rows","v":{"query":"rep","sections":[{"title":"Documents","rows":["report.pdf"],"hidden":2}],"selected":0,"preview":"file"}}"#,
        ),
        (
            Response::LauncherRows(LauncherRows {
                query: String::new(),
                sections: Vec::new(),
                selected: 0,
                preview: None,
            }),
            r#"{"kind":"launcher_rows","v":{"query":"","sections":[],"selected":0,"preview":null}}"#,
        ),
    ]
}

#[test]
fn every_request_round_trips_through_its_pinned_line() {
    for (request, wire) in requests() {
        let line = encode(&request).unwrap_or_else(|e| panic!("{request:?}: {e}"));
        assert_eq!(line, format!("{wire}\n"), "{request:?}");
        let back: Request = decode(&line).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert_eq!(back, request, "{line}");
    }
}

#[test]
fn every_response_round_trips_through_its_pinned_line() {
    for (response, wire) in responses() {
        let line = encode(&response).unwrap_or_else(|e| panic!("{response:?}: {e}"));
        assert_eq!(line, format!("{wire}\n"), "{response:?}");
        let back: Response = decode(&line).unwrap_or_else(|e| panic!("{line}: {e}"));
        assert_eq!(back, response, "{line}");
    }
}

#[test]
fn a_stats_reply_round_trips() {
    let response = Response::Stats(vec![stats(), stats()]);
    let line = encode(&response).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(line.matches('\n').count(), 1, "one line: {line}");
    let back: Response = decode(&line).unwrap_or_else(|e| panic!("{line}: {e}"));
    assert_eq!(back, response);
}

#[test]
fn a_message_survives_a_stream_and_leaves_the_rest_unread() {
    let mut wire = Vec::new();
    for (request, _) in requests() {
        write_message(&mut wire, &request).unwrap_or_else(|e| panic!("{e}"));
    }
    let mut reader = Cursor::new(wire);
    for (request, _) in requests() {
        let got: Request = read_message(&mut reader).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, request);
    }
    let end: Result<Request, IpcError> = read_message(&mut reader);
    assert!(matches!(end, Err(IpcError::Closed)), "{end:?}");
}

#[test]
fn a_malformed_line_is_refused() {
    const CASES: &[(&str, &str)] = &[
        ("empty", ""),
        ("not json", "launcher toggle"),
        ("unknown top-level kind", r#"{"kind":"explode"}"#),
        (
            "unknown nested kind",
            r#"{"kind":"show","v":{"kind":"launcher","v":{"kind":"explode"}}}"#,
        ),
        ("missing payload", r#"{"kind":"show"}"#),
        (
            "payload of the wrong shape",
            r#"{"kind":"act","v":{"kind":"dock","v":{"kind":"pin","v":{"app":7}}}}"#,
        ),
        (
            "internally tagged, not adjacently",
            r#"{"kind":"show","query":"x"}"#,
        ),
        (
            "two messages on one line",
            r#"{"kind":"ping"}{"kind":"quit"}"#,
        ),
        ("trailing garbage", r#"{"kind":"ping"} x"#),
        ("a bare string", r#""ping""#),
    ];
    for &(name, line) in CASES {
        let got: Result<Request, IpcError> = decode(line);
        assert!(
            matches!(got, Err(IpcError::Malformed(_))),
            "{name}: {line:?} gave {got:?}"
        );
    }
}

#[test]
fn an_overlong_line_is_refused_before_it_is_parsed() {
    let long = format!(
        r#"{{"kind":"show","v":{{"kind":"launcher","v":{{"kind":"open","v":{{"query":"{}"}}}}}}}}"#,
        "a".repeat(MAX_LINE)
    );
    let got: Result<Request, IpcError> = decode(&long);
    assert!(matches!(got, Err(IpcError::TooLong { .. })), "{got:?}");

    let mut reader = Cursor::new(format!("{long}\n").into_bytes());
    let streamed: Result<Request, IpcError> = read_message(&mut reader);
    assert!(
        matches!(streamed, Err(IpcError::TooLong { .. })),
        "{streamed:?}"
    );
}

#[test]
fn a_line_just_under_the_limit_is_accepted() {
    let request = Request::Show(ShowRequest::Launcher(LauncherRequest::Open {
        query: "a".repeat(MAX_LINE - 80),
    }));
    let line = encode(&request).unwrap_or_else(|e| panic!("{e}"));
    assert!(line.len() <= MAX_LINE, "{}", line.len());
    let mut reader = Cursor::new(line.into_bytes());
    let got: Request = read_message(&mut reader).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, request);
}
