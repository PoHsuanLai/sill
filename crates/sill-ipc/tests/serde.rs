//! Every request and response round-trips through one line, the wire spelling of each variant
//! is pinned, and a malformed line is refused rather than guessed at.

use std::io::Cursor;
use std::path::PathBuf;

use sill_ipc::{
    ControlCenterRequest, CornerName, DebugRequest, DesktopId, DisplayRequest, DockRequest,
    Failure, FailureKind, FakeAction, FakeCorner, FakeExpire, FakeMute, FakeNotification, FakeOsd,
    FakeScreenshot, FakeUrgency, IpcError, LauncherRequest, LevelRequest, MAX_LINE, Micros,
    NotificationsReport, NotificationsRequest, OsdTopic, Point, PointerButton, PowerMenuRequest,
    ReportHolder, ReportMode, Request, Response, ScreenshotRequest, ShotArea, SurfaceName,
    SurfaceStats, VolumeRequest, WidgetsRequest, decode, encode, read_message, write_message,
};

fn surface(name: &str) -> SurfaceName {
    SurfaceName(name.to_owned())
}

/// Every request variant, with the exact line it is written as.
fn requests() -> Vec<(Request, &'static str)> {
    vec![
        (Request::Ping, r#"{"kind":"ping"}"#),
        (
            Request::Launcher(LauncherRequest::Toggle),
            r#"{"kind":"launcher","v":{"kind":"toggle"}}"#,
        ),
        (
            Request::Launcher(LauncherRequest::Open {
                query: "fire".to_owned(),
            }),
            r#"{"kind":"launcher","v":{"kind":"open","v":{"query":"fire"}}}"#,
        ),
        (
            Request::Launcher(LauncherRequest::Open {
                query: String::new(),
            }),
            r#"{"kind":"launcher","v":{"kind":"open","v":{"query":""}}}"#,
        ),
        (
            Request::Launcher(LauncherRequest::Close),
            r#"{"kind":"launcher","v":{"kind":"close"}}"#,
        ),
        (
            Request::Dock(DockRequest::Pin {
                app: DesktopId("org.mozilla.firefox".to_owned()),
            }),
            r#"{"kind":"dock","v":{"kind":"pin","v":{"app":"org.mozilla.firefox"}}}"#,
        ),
        (
            Request::Dock(DockRequest::Unpin {
                app: DesktopId("foot".to_owned()),
            }),
            r#"{"kind":"dock","v":{"kind":"unpin","v":{"app":"foot"}}}"#,
        ),
        (Request::Reload, r#"{"kind":"reload"}"#),
        (Request::Quit, r#"{"kind":"quit"}"#),
        (Request::Lock, r#"{"kind":"lock"}"#),
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
            Request::Debug(DebugRequest::Osd(FakeOsd {
                topic: OsdTopic::Volume,
                percent: 40,
                mute: FakeMute::Muted,
            })),
            r#"{"kind":"debug","v":{"kind":"osd","v":{"topic":"volume","percent":40,"mute":"muted"}}}"#,
        ),
        (
            Request::Display(DisplayRequest::Brightness(LevelRequest::Up)),
            r#"{"kind":"display","v":{"kind":"brightness","v":{"kind":"up"}}}"#,
        ),
        (
            Request::Display(DisplayRequest::Brightness(LevelRequest::Set {
                percent: 40,
            })),
            r#"{"kind":"display","v":{"kind":"brightness","v":{"kind":"set","v":{"percent":40}}}}"#,
        ),
        (
            Request::Display(DisplayRequest::Volume(VolumeRequest::Level(
                LevelRequest::Down,
            ))),
            r#"{"kind":"display","v":{"kind":"volume","v":{"kind":"level","v":{"kind":"down"}}}}"#,
        ),
        (
            Request::Display(DisplayRequest::Volume(VolumeRequest::ToggleMute)),
            r#"{"kind":"display","v":{"kind":"volume","v":{"kind":"toggle_mute"}}}"#,
        ),
        (
            Request::PowerMenu(PowerMenuRequest::Open),
            r#"{"kind":"power_menu","v":{"kind":"open"}}"#,
        ),
        (
            Request::PowerMenu(PowerMenuRequest::Close),
            r#"{"kind":"power_menu","v":{"kind":"close"}}"#,
        ),
        (
            Request::PowerMenu(PowerMenuRequest::Toggle),
            r#"{"kind":"power_menu","v":{"kind":"toggle"}}"#,
        ),
        (
            Request::ControlCenter(ControlCenterRequest::Toggle),
            r#"{"kind":"control_center","v":{"kind":"toggle"}}"#,
        ),
        (
            Request::Notifications(NotificationsRequest::Toggle),
            r#"{"kind":"notifications","v":{"kind":"toggle"}}"#,
        ),
        (
            Request::Notifications(NotificationsRequest::Open),
            r#"{"kind":"notifications","v":{"kind":"open"}}"#,
        ),
        (
            Request::Notifications(NotificationsRequest::Close),
            r#"{"kind":"notifications","v":{"kind":"close"}}"#,
        ),
        (
            Request::Notifications(NotificationsRequest::DismissAll),
            r#"{"kind":"notifications","v":{"kind":"dismiss_all"}}"#,
        ),
        (
            Request::Notifications(NotificationsRequest::Status),
            r#"{"kind":"notifications","v":{"kind":"status"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Notify(FakeNotification {
                app: "Mail".to_owned(),
                summary: "Ada".to_owned(),
                body: "the <b>engine</b>".to_owned(),
                urgency: FakeUrgency::Critical,
                actions: vec![FakeAction {
                    key: "default".to_owned(),
                    label: "Open".to_owned(),
                }],
                expire: FakeExpire::Never,
                replaces: 0,
            })),
            r#"{"kind":"debug","v":{"kind":"notify","v":{"app":"Mail","summary":"Ada","body":"the <b>engine</b>","urgency":"critical","actions":[{"key":"default","label":"Open"}],"expire":"never","replaces":0}}}"#,
        ),
        (
            Request::Screenshot(ScreenshotRequest::Take(ShotArea::Area)),
            r#"{"kind":"screenshot","v":{"kind":"take","v":"area"}}"#,
        ),
        (
            Request::Screenshot(ScreenshotRequest::Take(ShotArea::Window)),
            r#"{"kind":"screenshot","v":{"kind":"take","v":"window"}}"#,
        ),
        (
            Request::Screenshot(ScreenshotRequest::Take(ShotArea::Screen)),
            r#"{"kind":"screenshot","v":{"kind":"take","v":"screen"}}"#,
        ),
        (
            Request::Widgets(WidgetsRequest::ToggleDesktop),
            r#"{"kind":"widgets","v":{"kind":"toggle_desktop"}}"#,
        ),
        (
            Request::Debug(DebugRequest::Screenshot(FakeScreenshot {
                path: PathBuf::from("/tmp/shots/Screenshot_1.png"),
            })),
            r#"{"kind":"debug","v":{"kind":"screenshot","v":{"path":"/tmp/shots/Screenshot_1.png"}}}"#,
        ),
        (
            Request::Debug(DebugRequest::Corner(FakeCorner {
                corner: CornerName::BottomRight,
            })),
            r#"{"kind":"debug","v":{"kind":"corner","v":{"corner":"bottom_right"}}}"#,
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
            Response::Failed(Failure {
                kind: FailureKind::UnknownSurface,
                message: "no surface sill-dock-HDMI-9".to_owned(),
            }),
            r#"{"kind":"failed","v":{"kind":"unknown_surface","message":"no surface sill-dock-HDMI-9"}}"#,
        ),
        (
            Response::Notifications(NotificationsReport {
                mode: ReportMode::Own,
                holder: ReportHolder::Other {
                    unique: ":1.42".to_owned(),
                    pid: Some(7),
                    command: Some("plasmashell".to_owned()),
                },
                banners: 2,
                kept: 9,
            }),
            r#"{"kind":"notifications","v":{"mode":"own","holder":{"kind":"other","v":{"unique":":1.42","pid":7,"command":"plasmashell"}},"banners":2,"kept":9}}"#,
        ),
        (
            Response::Notifications(NotificationsReport {
                mode: ReportMode::Passive,
                holder: ReportHolder::Sill,
                banners: 0,
                kept: 0,
            }),
            r#"{"kind":"notifications","v":{"mode":"passive","holder":{"kind":"sill"},"banners":0,"kept":0}}"#,
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
            r#"{"kind":"launcher","v":{"kind":"explode"}}"#,
        ),
        ("missing payload", r#"{"kind":"launcher"}"#),
        (
            "payload of the wrong shape",
            r#"{"kind":"dock","v":{"kind":"pin","v":{"app":7}}}"#,
        ),
        (
            "internally tagged, not adjacently",
            r#"{"kind":"launcher","query":"x"}"#,
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
        r#"{{"kind":"launcher","v":{{"kind":"open","v":{{"query":"{}"}}}}}}"#,
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
    let request = Request::Launcher(LauncherRequest::Open {
        query: "a".repeat(MAX_LINE - 64),
    });
    let line = encode(&request).unwrap_or_else(|e| panic!("{e}"));
    assert!(line.len() <= MAX_LINE, "{}", line.len());
    let mut reader = Cursor::new(line.into_bytes());
    let got: Request = read_message(&mut reader).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, request);
}
