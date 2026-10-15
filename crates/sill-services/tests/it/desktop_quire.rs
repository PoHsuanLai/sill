//! The Quire arm over desktop-client's fake: a scripted `quire_desktop_v1` session becomes the
//! shell's view (apps, windows, Spaces, what is active), the shell's commands become Quire
//! requests, the shell protocol's events become notes, and the handed connection comes from a
//! stand-in `org.quire.Compositor1` on a private bus (never the person's session bus).

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use casement_protocols::vocab::{
    ActorKind, AppLifeKind, Corner, Emphasis, KeyGesture, MacMods, OverviewMode, OverviewState,
    PlaceTarget, ProofKind, ScreenEdge, ShellRole, Shown, SpaceKind, SplitSide, Toggle,
    WindowEntry, WindowStateKind,
};
use desktop_client::capture::{CaptureEvent, ShmFormat};
use desktop_client::fake::FakeDesktop;
use desktop_client::quire::{
    AppChange, AppEvent, MenuSource, QuireEvent, ShellEvent, SpaceChange, SpaceEvent, SpacePlace,
    WindowChange, WindowEvent,
};
use desktop_client::toplevel::{ToplevelChange, ToplevelEvent, ToplevelFlags};
use desktop_client::workspace::{
    GroupChange, WorkspaceAbility, WorkspaceChange, WorkspaceEvent, WorkspaceFlag,
};
use desktop_client::{
    AppId, AppKey, CaptureRequest, CommandError, Desktop, DesktopCommand as Wire, DesktopConfig,
    DesktopEvent, GroupKey, OutputName, Point, Protocol, ProtocolSet, Rect, Size,
    SpaceId as WireSpaceId, SpaceKey, SpaceTarget, SurfaceKey, ToplevelIdentifier, ToplevelKey,
    WorkspaceKey,
};
use porter_core::SpaceId;
use sill_model::desktop::{DesktopCommand, OverviewNote, ShellNote, SpacePlaceView, SwitcherNote};
use sill_services::desktop_wl::Refusal;
use sill_services::desktop_wl::quire::{
    ArmError, ConnectionRole, FakeHandout, Handout, HandoutError, QuireArm, role_of_surface,
    shell_connection,
};
use sill_services::env::{Bus, BusRoute};
use sill_testkit::PrivateBus;

fn quire_offer() -> ProtocolSet {
    [
        Protocol::ExtForeignToplevelList,
        Protocol::ExtWorkspace,
        Protocol::QuireDesktop,
        Protocol::QuireShell,
    ]
    .into_iter()
    .fold(ProtocolSet::empty(), ProtocolSet::with)
}

fn tk(n: u64) -> ToplevelKey {
    ToplevelKey::new(n)
}

fn ak(n: u64) -> AppKey {
    AppKey::new(n)
}

fn sk(n: u64) -> SpaceKey {
    SpaceKey::new(n)
}

fn quire(event: QuireEvent) -> DesktopEvent {
    DesktopEvent::Quire(event)
}

fn app(n: u64, id: &str, mru: u32) -> Vec<DesktopEvent> {
    let change = |change| quire(QuireEvent::App(AppEvent::Changed { key: ak(n), change }));
    vec![
        quire(QuireEvent::App(AppEvent::Opened(ak(n)))),
        change(AppChange::Identity {
            id: AppId::new(id),
            proof: ProofKind::Scope,
            claimed_id: id.to_owned(),
        }),
        change(AppChange::Life(AppLifeKind::Running)),
        change(AppChange::Menu(MenuSource::none())),
        change(AppChange::Mru(mru)),
        quire(QuireEvent::App(AppEvent::Done(ak(n)))),
    ]
}

fn space(n: u64, id: &str, position: u32, output: &str) -> Vec<DesktopEvent> {
    let change = |change| {
        quire(QuireEvent::Space(SpaceEvent::Changed {
            key: sk(n),
            change,
        }))
    };
    vec![
        quire(QuireEvent::Space(SpaceEvent::Opened(sk(n)))),
        change(SpaceChange::Id(WireSpaceId::new(id))),
        change(SpaceChange::Kind(SpaceKind::Desktop)),
        change(SpaceChange::Position(position)),
        change(SpaceChange::Output(OutputName::new(output))),
        quire(QuireEvent::Space(SpaceEvent::Done(sk(n)))),
    ]
}

/// A window: its ext toplevel (identifier `id-<n>`, title, app id) and its Quire window.
fn window(
    n: u64,
    app: u64,
    app_id: &str,
    on: SpacePlace,
    state: WindowStateKind,
) -> Vec<DesktopEvent> {
    let top = |change| DesktopEvent::Toplevel(ToplevelEvent::Changed { key: tk(n), change });
    let win = |change| {
        quire(QuireEvent::Window(WindowEvent::Changed {
            key: tk(n),
            change,
        }))
    };
    vec![
        DesktopEvent::Toplevel(ToplevelEvent::Opened(tk(n))),
        top(ToplevelChange::Identifier(ToplevelIdentifier::new(
            format!("id-{n}"),
        ))),
        top(ToplevelChange::AppId(app_id.to_owned())),
        top(ToplevelChange::Title(format!("{app_id} {n}"))),
        top(ToplevelChange::Flags(ToplevelFlags::of([]))),
        DesktopEvent::Toplevel(ToplevelEvent::Done(tk(n))),
        quire(QuireEvent::Window(WindowEvent::Opened(tk(n)))),
        win(WindowChange::App(ak(app))),
        win(WindowChange::State {
            kind: state,
            actor: ActorKind::User,
        }),
        win(WindowChange::Place(on)),
        win(WindowChange::Output(OutputName::new("DP-1"))),
        win(WindowChange::Emphasis(if n == 1 {
            Emphasis::Key
        } else {
            Emphasis::Inactive
        })),
        quire(QuireEvent::Window(WindowEvent::Done(tk(n)))),
    ]
}

/// The ext workspace behind the Space `desktop`, active: what `space_shown` reads.
fn desktop_workspace() -> Vec<DesktopEvent> {
    let group = GroupKey::new(1);
    let key = WorkspaceKey::new(10);
    let change = |change| DesktopEvent::Workspace(WorkspaceEvent::Changed { key, change });
    vec![
        DesktopEvent::Workspace(WorkspaceEvent::GroupOpened(group)),
        DesktopEvent::Workspace(WorkspaceEvent::Group {
            key: group,
            change: GroupChange::OutputEntered(OutputName::new("DP-1")),
        }),
        DesktopEvent::Workspace(WorkspaceEvent::Opened(key)),
        change(WorkspaceChange::Id("desktop".to_owned())),
        change(WorkspaceChange::Flags(BTreeSet::from([
            WorkspaceFlag::Active,
        ]))),
        change(WorkspaceChange::Abilities(BTreeSet::from([
            WorkspaceAbility::Activate,
        ]))),
        change(WorkspaceChange::GroupEntered(group)),
        DesktopEvent::Workspace(WorkspaceEvent::Done),
    ]
}

/// Two apps, two Spaces, two windows (the first key on `desktop`, the second minimised on
/// `two`), the first active.
fn session(config: DesktopConfig) -> FakeDesktop {
    let mut desktop = FakeDesktop::with_config(quire_offer(), config);
    let events = [
        app(1, "org.example.term", 0),
        app(2, "org.example.notes", 1),
        space(1, "desktop", 0, "DP-1"),
        space(2, "two", 1, "DP-1"),
        desktop_workspace(),
        window(
            1,
            1,
            "org.example.term",
            SpacePlace::On(sk(1)),
            WindowStateKind::Floating,
        ),
        window(
            2,
            2,
            "org.example.notes",
            SpacePlace::On(sk(2)),
            WindowStateKind::Minimised,
        ),
        vec![quire(QuireEvent::Active {
            app: Some(ak(1)),
            key: Some(tk(1)),
            actor: ActorKind::User,
        })],
    ];
    events.into_iter().flatten().for_each(|e| desktop.script(e));
    desktop.advance_all();
    desktop
}

fn shell_arm() -> QuireArm<FakeDesktop, Vec<ShellNote>> {
    QuireArm::new(session(DesktopConfig::holding_shell()), Vec::new())
}

fn space_id(text: &str) -> SpaceId {
    SpaceId::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_view_names_apps_windows_spaces_and_the_active_pair() {
    let mut arm = shell_arm();
    let view = arm.sync();

    let apps: Vec<(&str, u32)> = view.apps.iter().map(|a| (a.key.as_str(), a.mru)).collect();
    assert_eq!(apps, [("org.example.term", 0), ("org.example.notes", 1)]);
    assert!(view.apps.iter().all(|a| a.proof == ProofKind::Scope));

    let windows: Vec<(&str, &str, WindowStateKind, &SpacePlaceView)> = view
        .windows
        .iter()
        .map(|w| (w.identifier.as_str(), w.app.as_str(), w.state, &w.space))
        .collect();
    assert_eq!(
        windows,
        [
            (
                "id-1",
                "org.example.term",
                WindowStateKind::Floating,
                &SpacePlaceView::On(space_id("desktop"))
            ),
            (
                "id-2",
                "org.example.notes",
                WindowStateKind::Minimised,
                &SpacePlaceView::On(space_id("two"))
            ),
        ]
    );
    assert_eq!(view.windows[0].emphasis, Emphasis::Key);
    assert_eq!(view.windows[0].output, "DP-1");

    let spaces: Vec<(String, u32, Shown)> = view
        .spaces
        .iter()
        .map(|s| (s.id.to_string(), s.position, s.shown))
        .collect();
    assert_eq!(
        spaces,
        [
            ("desktop".to_owned(), 0, Shown::Shown),
            ("two".to_owned(), 1, Shown::Hidden)
        ],
        "a Space is shown when its ext workspace is active"
    );

    assert_eq!(view.active.app.as_deref(), Some("org.example.term"));
    assert_eq!(view.active.key_window.as_deref(), Some("id-1"));
}

#[test]
fn the_shells_commands_become_quire_requests() {
    let mut arm = shell_arm();
    let sent = |arm: &mut QuireArm<FakeDesktop, Vec<ShellNote>>, command: DesktopCommand| {
        arm.execute(&command)
            .unwrap_or_else(|e| panic!("{command:?}: {e}"));
        arm.desktop_mut().take_sent()
    };
    let win = || "id-1".to_owned();
    let term = || "org.example.term".to_owned();
    let cases: Vec<(&str, DesktopCommand, Vec<Wire>)> = vec![
        (
            "place",
            DesktopCommand::Place(win(), PlaceTarget::Left),
            vec![Wire::Place {
                toplevel: tk(1),
                target: PlaceTarget::Left,
            }],
        ),
        (
            "to a Space",
            DesktopCommand::ToSpace(win(), space_id("two")),
            vec![Wire::MoveToSpace {
                toplevel: tk(1),
                space: sk(2),
            }],
        ),
        (
            "switch Space",
            DesktopCommand::SwitchSpace(space_id("two")),
            vec![Wire::ActivateSpace(sk(2))],
        ),
        (
            "activating a minimised window restores it first",
            DesktopCommand::Activate("id-2".to_owned()),
            vec![Wire::Unminimise(tk(2)), Wire::Activate(tk(2))],
        ),
        (
            "hide an app",
            DesktopCommand::HideApp(term()),
            vec![Wire::HideApp(ak(1))],
        ),
        (
            "quit an app",
            DesktopCommand::QuitApp(term()),
            vec![Wire::QuitApp(ak(1))],
        ),
        (
            "reopen an app",
            DesktopCommand::ReopenApp(term()),
            vec![Wire::ReopenApp(ak(1))],
        ),
        (
            "create a Space",
            DesktopCommand::CreateSpace("DP-1".to_owned()),
            vec![Wire::CreateSpace {
                output: OutputName::new("DP-1"),
            }],
        ),
        (
            "remove a Space",
            DesktopCommand::RemoveSpace(space_id("two")),
            vec![Wire::RemoveSpace(sk(2))],
        ),
        (
            "move a Space",
            DesktopCommand::MoveSpace {
                space: space_id("two"),
                position: 0,
            },
            vec![Wire::MoveSpace {
                space: sk(2),
                position: 0,
            }],
        ),
        (
            "open Mission Control",
            DesktopCommand::Overview {
                mode: OverviewMode::MissionControl,
                toggle: Toggle::On,
            },
            vec![Wire::Overview {
                mode: OverviewMode::MissionControl,
                toggle: Toggle::On,
            }],
        ),
        (
            "show the desktop",
            DesktopCommand::ShowDesktop(Toggle::On),
            vec![Wire::ShowDesktop(Toggle::On)],
        ),
        (
            "pick in the switcher",
            DesktopCommand::PickSwitcher(2),
            vec![Wire::PickSwitcher(2)],
        ),
        (
            "cancel the switcher",
            DesktopCommand::CancelSwitcher,
            vec![Wire::CancelSwitcher],
        ),
        (
            "pick Left in a window's menu",
            DesktopCommand::PickWindowMenu("id-1".to_owned(), WindowEntry::Left),
            vec![Wire::PickWindowMenu {
                window: ToplevelIdentifier::new("id-1"),
                entry: WindowEntry::Left,
            }],
        ),
        (
            "pick a window for the other half",
            DesktopCommand::PickSplit("id-2".to_owned()),
            vec![Wire::PickSplit(ToplevelIdentifier::new("id-2"))],
        ),
        (
            "turn the Split View down",
            DesktopCommand::CancelSplit,
            vec![Wire::CancelSplit],
        ),
        (
            "move the divider",
            DesktopCommand::DivideSplit("id-1".to_owned(), 400),
            vec![Wire::DivideSplit {
                window: ToplevelIdentifier::new("id-1"),
                share: 400,
            }],
        ),
        (
            "set the Space drop targets",
            DesktopCommand::SetSpaceTargets {
                output: "DP-1".to_owned(),
                targets: vec![(space_id("two"), (10, 12, 160, 100))],
            },
            vec![Wire::SetSpaceTargets {
                output: OutputName::new("DP-1"),
                targets: vec![SpaceTarget::new(
                    WireSpaceId::new("two"),
                    Rect::new(10, 12, 160, 100),
                )],
            }],
        ),
    ];
    for (name, command, want) in cases {
        assert_eq!(sent(&mut arm, command), want, "{name}");
    }
}

#[test]
fn a_command_for_something_gone_is_refused_before_anything_is_sent() {
    let mut arm = shell_arm();
    let refusals = [
        ("a window", DesktopCommand::Close("id-99".to_owned())),
        (
            "an app",
            DesktopCommand::QuitApp("org.example.none".to_owned()),
        ),
        ("a Space", DesktopCommand::SwitchSpace(space_id("nowhere"))),
    ];
    for (name, command) in refusals {
        let refused = arm.execute(&command);
        assert!(
            matches!(
                refused,
                Err(ArmError::Refused(
                    Refusal::UnknownWindow(_) | Refusal::UnknownApp(_) | Refusal::UnknownSpace(_)
                ))
            ),
            "{name}: {refused:?}"
        );
    }
    assert!(arm.desktop_mut().take_sent().is_empty());
}

#[test]
fn a_minimise_target_names_the_docks_surface() {
    let mut arm = shell_arm();
    let target = DesktopCommand::SetMinimiseTarget {
        toplevel: "id-2".to_owned(),
        rect: (4, 8, 64, 64),
    };
    assert!(
        matches!(
            arm.execute(&target),
            Err(ArmError::Refused(Refusal::NoSurface(ShellRole::Dock)))
        ),
        "no dock surface is registered yet"
    );

    arm.assign(ShellRole::Dock, "DP-1", SurfaceKey::new(7))
        .unwrap_or_else(|e| panic!("{e}"));
    arm.execute(&target).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        arm.desktop_mut().take_sent(),
        [
            Wire::SetShellRole {
                surface: SurfaceKey::new(7),
                role: ShellRole::Dock,
            },
            Wire::SetMinimiseTarget {
                toplevel: tk(2),
                surface: SurfaceKey::new(7),
                rect: Rect::new(4, 8, 64, 64),
            },
        ]
    );
}

#[test]
fn a_client_without_the_shell_role_cannot_send_the_shells_commands() {
    let mut arm = QuireArm::new(session(DesktopConfig::default()), Vec::<ShellNote>::new());
    let refused = arm.execute(&DesktopCommand::PickSwitcher(0));
    assert!(
        matches!(refused, Err(ArmError::Send(CommandError::NotBound { .. }))),
        "{refused:?}"
    );
    arm.execute(&DesktopCommand::QuitApp("org.example.term".to_owned()))
        .unwrap_or_else(|e| panic!("an app command needs no shell role: {e}"));
}

#[test]
fn the_shell_protocols_events_become_notes_in_order() {
    let mut arm = shell_arm();
    let _settled = arm.sync();
    let say = |arm: &mut QuireArm<FakeDesktop, Vec<ShellNote>>, events: Vec<ShellEvent>| {
        for event in events {
            arm.desktop_mut().script(quire(QuireEvent::Shell(event)));
        }
        arm.desktop_mut().advance_all();
        arm.sync()
    };
    let term = || AppId::new("org.example.term");
    let view = say(
        &mut arm,
        vec![
            ShellEvent::Modifiers {
                surface: None,
                mods: MacMods::OPTION,
            },
            ShellEvent::KeyGesture(KeyGesture::Summon),
            ShellEvent::SwitcherApp {
                index: 0,
                app: term(),
            },
            ShellEvent::SwitcherShown { selected: 0 },
            ShellEvent::SwitcherSelected(1),
            ShellEvent::SwitcherHidden,
            ShellEvent::Overview {
                mode: OverviewMode::MissionControl,
                state: OverviewState::Open,
                progress: 1000,
            },
            ShellEvent::OverviewSlot {
                window: ToplevelIdentifier::new("id-1"),
                rect: Rect::new(0, 0, 100, 100),
            },
            ShellEvent::OverviewDone,
            ShellEvent::HotCorner(Corner::TopRight),
            ShellEvent::Reveal {
                edge: ScreenEdge::Bottom,
                shown: Shown::Shown,
            },
            ShellEvent::MenuFlash {
                app: term(),
                item: 3,
            },
            ShellEvent::Bell(term()),
            ShellEvent::Reopen(term()),
            ShellEvent::WindowMenu {
                window: ToplevelIdentifier::new("id-1"),
                at: Point::new(40, 52),
                enabled: 0b1100,
            },
            ShellEvent::SplitChoose {
                first: ToplevelIdentifier::new("id-1"),
                side: SplitSide::Left,
            },
            ShellEvent::SplitCandidate(ToplevelIdentifier::new("id-2")),
            ShellEvent::SplitChoiceEnd(ToplevelIdentifier::new("id-1")),
            ShellEvent::OverviewOutput {
                output: OutputName::new("DP-1"),
                shown: Shown::Shown,
            },
            ShellEvent::DropTarget {
                output: OutputName::new("DP-1"),
                space: Some(WireSpaceId::new("two")),
            },
            ShellEvent::DropTarget {
                output: OutputName::new("DP-1"),
                space: None,
            },
            ShellEvent::Killed,
        ],
    );
    assert_eq!(
        arm.sink(),
        &[
            ShellNote::Modifiers(MacMods::OPTION),
            ShellNote::Gesture(KeyGesture::Summon),
            ShellNote::Switcher(SwitcherNote::App {
                index: 0,
                app: "org.example.term".to_owned()
            }),
            ShellNote::Switcher(SwitcherNote::Shown { selected: 0 }),
            ShellNote::Switcher(SwitcherNote::Selected(1)),
            ShellNote::Switcher(SwitcherNote::Hidden),
            ShellNote::Overview(OverviewNote::Moved {
                mode: OverviewMode::MissionControl,
                state: OverviewState::Open,
                progress: 1000
            }),
            ShellNote::Overview(OverviewNote::Laid),
            ShellNote::HotCorner(Corner::TopRight),
            ShellNote::Reveal {
                edge: ScreenEdge::Bottom,
                shown: Shown::Shown
            },
            ShellNote::MenuFlash {
                app: "org.example.term".to_owned(),
                item: 3
            },
            ShellNote::Bell("org.example.term".to_owned()),
            ShellNote::Reopen("org.example.term".to_owned()),
            ShellNote::WindowMenu {
                window: "id-1".to_owned(),
                at: (40, 52),
                enabled: 0b1100,
            },
            ShellNote::SplitChoose {
                first: "id-1".to_owned(),
                side: SplitSide::Left,
            },
            ShellNote::SplitCandidate("id-2".to_owned()),
            ShellNote::SplitChoiceEnd("id-1".to_owned()),
            ShellNote::OverviewOutput {
                output: "DP-1".to_owned(),
                shown: Shown::Shown
            },
            ShellNote::DropTarget {
                output: "DP-1".to_owned(),
                space: Some(space_id("two"))
            },
            ShellNote::DropTarget {
                output: "DP-1".to_owned(),
                space: None
            },
            ShellNote::Killed,
        ],
        "the layout's slot is state, not a note"
    );
    assert_eq!(view.windows.len(), 2, "the view is still the desktop's");
}

#[test]
fn the_shells_surfaces_have_the_roles_the_compositor_knows() {
    let cases = [
        ("bar", Some(ShellRole::Bar)),
        ("dock", Some(ShellRole::Dock)),
        ("launcher", Some(ShellRole::Launcher)),
        ("switcher", Some(ShellRole::Switcher)),
        ("banners", Some(ShellRole::Notifications)),
        ("osd", Some(ShellRole::Osd)),
        ("wallpaper", Some(ShellRole::Wallpaper)),
        ("polkit", None),
        ("launcher-catcher", None),
    ];
    for (name, want) in cases {
        assert_eq!(role_of_surface(name), want, "{name}");
    }
}

#[tokio::test]
async fn a_fake_handout_gives_its_socket_once() {
    let (handed, mut ours) = UnixStream::pair().unwrap_or_else(|e| panic!("{e}"));
    let handout = FakeHandout::handing(OwnedFd::from(handed));
    let fd = handout
        .connect(ConnectionRole::Shell)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    UnixStream::from(fd)
        .write_all(b"x")
        .unwrap_or_else(|e| panic!("{e}"));
    let mut byte = [0u8; 1];
    ours.read_exact(&mut byte).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&byte, b"x");
    let again = handout.connect(ConnectionRole::Shell).await;
    assert!(matches!(again, Err(HandoutError::Nothing)));
}

/// `org.quire.Compositor1` as far as the shell uses it: each `Connect` records its role and
/// hands over one end of a socket pair, keeping the other.
struct Compositor {
    roles: Arc<Mutex<Vec<String>>>,
    kept: Arc<Mutex<Vec<UnixStream>>>,
}

#[zbus::interface(name = "org.quire.Compositor1")]
impl Compositor {
    fn connect(&self, role: &str) -> zbus::fdo::Result<zbus::zvariant::OwnedFd> {
        let (handed, ours) =
            UnixStream::pair().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        if let Ok(mut roles) = self.roles.lock() {
            roles.push(role.to_owned());
        }
        if let Ok(mut kept) = self.kept.lock() {
            kept.push(ours);
        }
        Ok(OwnedFd::from(handed).into())
    }
}

#[tokio::test]
async fn the_compositor_on_the_bus_hands_the_shell_a_connected_socket() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let route = Bus::session(BusRoute::Address(bus.address.clone()));
    assert!(
        shell_connection(&route).await.is_none(),
        "no compositor owns the name: the environment route stays"
    );

    let roles = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::new(Mutex::new(Vec::new()));
    let _compositor = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(
            "/org/quire/Compositor1",
            Compositor {
                roles: Arc::clone(&roles),
                kept: Arc::clone(&kept),
            },
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .name("org.quire.Compositor1")
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("the stand-in compositor: {e}"));

    let fd = shell_connection(&route)
        .await
        .unwrap_or_else(|| panic!("the stand-in hands a socket"));
    assert_eq!(
        roles.lock().map(|roles| roles.clone()).unwrap_or_default(),
        ["shell"]
    );
    UnixStream::from(fd)
        .write_all(b"y")
        .unwrap_or_else(|e| panic!("{e}"));
    let mut ours = kept
        .lock()
        .unwrap_or_else(|e| panic!("{e}"))
        .pop()
        .unwrap_or_else(|| panic!("the stand-in kept its end"));
    let mut byte = [0u8; 1];
    ours.read_exact(&mut byte).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&byte, b"y");
}

/// A shell arm whose compositor also offers the standard capture protocol.
fn capture_arm() -> QuireArm<FakeDesktop, Vec<ShellNote>> {
    let offer = quire_offer().with(Protocol::ExtImageCopyCapture);
    let desktop = FakeDesktop::with_config(offer, DesktopConfig::holding_shell());
    QuireArm::new(desktop, Vec::new())
}

#[test]
fn a_picture_of_a_space_is_asked_for_by_output_and_comes_back_as_a_png_note() {
    use desktop_client::capture::CapturePhase;
    let mut arm = capture_arm();
    arm.execute(&DesktopCommand::CaptureSpace {
        output: "DP-1".to_owned(),
        space: space_id("two"),
        size: (320, 200),
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        arm.desktop_mut().take_sent(),
        [Wire::CaptureOutput(CaptureRequest::of_space(
            OutputName::new("DP-1"),
            WireSpaceId::new("two"),
            Size::new(320, 200),
        ))]
    );
    // Still going: nothing is said.
    arm.pictures(|_, _| None);
    assert!(arm.sink().is_empty());
    // Finished: one note, once.
    let mut finished = |arm: &mut QuireArm<FakeDesktop, Vec<ShellNote>>| {
        arm.pictures(|desktop, id| {
            finish_capture_in(desktop, id);
            match desktop.state().captures.get(id).map(|s| s.phase.clone()) {
                Some(CapturePhase::Ready(frame)) => Some(Ok(frame)),
                _ => None,
            }
        });
    };
    finished(&mut arm);
    finished(&mut arm);
    let [
        ShellNote::SpacePicture {
            output,
            space,
            picture,
        },
    ] = arm.sink().as_slice()
    else {
        panic!("one picture note: {:?}", arm.sink());
    };
    assert_eq!((output.as_str(), space), ("DP-1", &space_id("two")));
    let sill_model::desktop::SpacePicture::Png(bytes) = picture else {
        panic!("a PNG: {picture:?}");
    };
    let decoded = image::load_from_memory(bytes)
        .unwrap_or_else(|e| panic!("{e}"))
        .to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (2, 1));
    assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
}

/// [`finish_capture`] on a bare desktop.
fn finish_capture_in(desktop: &mut FakeDesktop, id: desktop_client::CaptureId) {
    let pixel = [[0_u8, 0, 255, 255], [255, 0, 0, 255]].concat();
    for event in [
        CaptureEvent::BufferSize(Size::new(2, 1)),
        CaptureEvent::Format(ShmFormat::ARGB8888),
        CaptureEvent::ConstraintsDone,
        CaptureEvent::Ready {
            stride: 8,
            pixels: pixel,
        },
    ] {
        desktop.script(DesktopEvent::Capture { id, event });
    }
    desktop.advance_all();
}
