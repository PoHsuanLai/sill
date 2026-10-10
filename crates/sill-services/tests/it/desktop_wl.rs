//! The standard-protocol arm over desktop-client's fake: a scripted ext/wlr session becomes the
//! windows and workspaces the shell draws (recency, sizing, workspaces), the desktop view
//! (apps, Spaces, what is active), the requests the shell's commands become, the displays'
//! heads and scale configuration, and the arm the globals choose.

use std::collections::BTreeSet;

use casement_protocols::vocab::{PlaceTarget, Shown};
use desktop_client::fake::FakeDesktop;
use desktop_client::output::{
    Enabled, HeadChange, HeadChangeRequest, ModeChange, OutputEvent, RefreshMilliHz, ScaleMilli,
};
use desktop_client::toplevel::{ToplevelChange, ToplevelEvent, ToplevelFlag, ToplevelFlags};
use desktop_client::workspace::{
    GroupChange, WorkspaceAbility, WorkspaceChange, WorkspaceEvent, WorkspaceFlag,
};
use desktop_client::{
    Desktop, DesktopCommand as Wire, DesktopEvent, DesktopState, GroupKey, HeadKey, ModeKey,
    OutputName as WireOutput, Point, Protocol, ProtocolSet, Size, ToplevelIdentifier, ToplevelKey,
    WorkspaceKey as WireWorkspaceKey,
};
use sill_launcher::WindowKey;
use sill_model::apps::AppIndex;
use sill_model::cosmic_wl::{Activity, Sizing, Toplevels, WindowState, WlCommand};
use sill_model::desktop::{
    DesktopBackend, DesktopCommand, GlobalOffer, SpacePlaceView, Supports, supports,
};
use sill_model::displays::randr::Enablement;
use sill_model::displays::scale::DisplayScale;
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::keyboard::focused;
use sill_services::desktop_wl::{
    Refusal, ScaleRefusal, backend_of, config_for_scale, desktop_of, desktop_view, heads_of,
    offer_of_protocols, project_toplevels, project_workspaces, space_id, wire_of,
};

fn full_offer() -> ProtocolSet {
    [
        Protocol::ExtForeignToplevelList,
        Protocol::WlrForeignToplevelManagement,
        Protocol::ExtWorkspace,
        Protocol::WlrOutputManagement,
    ]
    .into_iter()
    .fold(ProtocolSet::empty(), ProtocolSet::with)
}

/// A compositor that offers the standard protocols, and says what a script tells it.
struct Session {
    desktop: FakeDesktop,
}

impl Session {
    fn new() -> Session {
        Session {
            desktop: FakeDesktop::new(full_offer()),
        }
    }

    fn say(&mut self, events: impl IntoIterator<Item = DesktopEvent>) -> &mut Session {
        for event in events {
            self.desktop.script(event);
        }
        self.desktop.advance_all();
        self
    }

    fn state(&self) -> &DesktopState {
        self.desktop.state()
    }
}

fn tk(n: u64) -> ToplevelKey {
    ToplevelKey::new(n)
}

fn wk(n: u64) -> WireWorkspaceKey {
    WireWorkspaceKey::new(n)
}

fn output(name: &str) -> WireOutput {
    WireOutput::new(name)
}

fn change(key: ToplevelKey, change: ToplevelChange) -> DesktopEvent {
    DesktopEvent::Toplevel(ToplevelEvent::Changed { key, change })
}

/// A window appearing complete: the list's identifier, then the managed state.
fn opens(key: u64, app: &str, flags: &[ToplevelFlag], on: &str) -> Vec<DesktopEvent> {
    vec![
        DesktopEvent::Toplevel(ToplevelEvent::Opened(tk(key))),
        change(
            tk(key),
            ToplevelChange::Identifier(ToplevelIdentifier::new(format!("id-{key}"))),
        ),
        change(tk(key), ToplevelChange::AppId(app.to_owned())),
        change(tk(key), ToplevelChange::Title(format!("{app} {key}"))),
        change(tk(key), ToplevelChange::OutputEntered(output(on))),
        change(
            tk(key),
            ToplevelChange::Flags(ToplevelFlags::of(flags.iter().copied())),
        ),
        DesktopEvent::Toplevel(ToplevelEvent::Done(tk(key))),
    ]
}

fn flags(key: u64, flags: &[ToplevelFlag]) -> DesktopEvent {
    change(
        tk(key),
        ToplevelChange::Flags(ToplevelFlags::of(flags.iter().copied())),
    )
}

fn closes(key: u64) -> DesktopEvent {
    DesktopEvent::Toplevel(ToplevelEvent::Closed(tk(key)))
}

/// One group on `output` with these workspaces `(key, name, id, position, active)`, announced
/// in the order given.
fn group_with(
    group: u64,
    on: &str,
    workspaces: &[(u64, &str, &str, u32, bool)],
) -> Vec<DesktopEvent> {
    let gk = GroupKey::new(group);
    let mut events = vec![
        DesktopEvent::Workspace(WorkspaceEvent::GroupOpened(gk)),
        DesktopEvent::Workspace(WorkspaceEvent::Group {
            key: gk,
            change: GroupChange::OutputEntered(output(on)),
        }),
    ];
    for (key, name, id, position, active) in workspaces {
        let key = wk(*key);
        let state: BTreeSet<WorkspaceFlag> = active
            .then_some(WorkspaceFlag::Active)
            .into_iter()
            .collect();
        let abilities: BTreeSet<WorkspaceAbility> = [WorkspaceAbility::Activate].into();
        let changes = [
            WorkspaceChange::Name((*name).to_owned()),
            WorkspaceChange::Id((*id).to_owned()),
            WorkspaceChange::Coordinates(vec![*position]),
            WorkspaceChange::Flags(state),
            WorkspaceChange::Abilities(abilities),
            WorkspaceChange::GroupEntered(gk),
        ];
        events.push(DesktopEvent::Workspace(WorkspaceEvent::Opened(key)));
        events.extend(
            changes
                .into_iter()
                .map(|change| DesktopEvent::Workspace(WorkspaceEvent::Changed { key, change })),
        );
    }
    events.push(DesktopEvent::Workspace(WorkspaceEvent::Done));
    events
}

fn order(toplevels: &Toplevels) -> Vec<u64> {
    toplevels.windows.iter().map(|w| w.key.0).collect()
}

use ToplevelFlag::{Activated, Fullscreen, Maximized, Minimized};

/// A shell command, what it should become on the wire, and its name.
type WireCase = (&'static str, WlCommand, Result<Vec<Wire>, Refusal>);

#[test]
fn windows_follow_recency_as_they_activate_and_close() {
    let mut session = Session::new();
    session
        .say(opens(1, "term", &[], "DP-1"))
        .say(opens(2, "files", &[], "DP-1"))
        .say(opens(3, "browser", &[], "DP-1"));
    let opened = project_toplevels(Toplevels::default(), session.state());
    assert_eq!(
        order(&opened),
        [1, 2, 3],
        "inactive windows keep opening order"
    );

    session.say([flags(3, &[Activated])]);
    let third = project_toplevels(opened, session.state());
    assert_eq!(order(&third), [3, 1, 2], "the activated window goes first");

    session.say([flags(3, &[]), flags(2, &[Activated])]);
    let second = project_toplevels(third, session.state());
    assert_eq!(order(&second), [2, 3, 1]);

    session.say([closes(3)]);
    let closed = project_toplevels(second, session.state());
    assert_eq!(
        order(&closed),
        [2, 1],
        "a closed window leaves, the rest keep their order"
    );
}

#[test]
fn a_window_is_described_by_its_flags_and_output() {
    let mut session = Session::new();
    session
        .say(opens(1, "a", &[Minimized], "DP-1"))
        .say(opens(2, "b", &[Maximized], "DP-2"))
        .say(opens(3, "c", &[Maximized, Fullscreen, Activated], "DP-1"));
    let all = project_toplevels(Toplevels::default(), session.state());
    let of = |key: u64| {
        all.windows
            .iter()
            .find(|w| w.key == WindowKey(key))
            .map(|w| (w.activity, w.window, w.sizing, w.outputs.clone()))
            .unwrap_or_else(|| panic!("window {key}"))
    };
    let out = |name: &str| vec![OutputName(name.to_owned())];
    let cases = [
        (
            "minimised",
            1,
            (
                Activity::Inactive,
                WindowState::Minimized,
                Sizing::Floating,
                out("DP-1"),
            ),
        ),
        (
            "maximised",
            2,
            (
                Activity::Inactive,
                WindowState::Normal,
                Sizing::Maximized,
                out("DP-2"),
            ),
        ),
        (
            "full screen over maximised",
            3,
            (
                Activity::Active,
                WindowState::Normal,
                Sizing::Fullscreen,
                out("DP-1"),
            ),
        ),
    ];
    for (name, key, want) in cases {
        assert_eq!(of(key), want, "{name}");
    }
    assert_eq!(order(&all), [3, 1, 2], "the active window is first");
    assert_eq!(all.focused_output(), Some(OutputName("DP-1".to_owned())));
}

#[test]
fn a_half_described_window_is_not_shown() {
    let mut session = Session::new();
    session.say([
        DesktopEvent::Toplevel(ToplevelEvent::Opened(tk(1))),
        change(tk(1), ToplevelChange::AppId("term".to_owned())),
    ]);
    assert_eq!(
        order(&project_toplevels(Toplevels::default(), session.state())),
        Vec::<u64>::new()
    );
}

#[test]
fn workspaces_are_grouped_and_in_position_order() {
    let mut session = Session::new();
    // Announced out of order: the position decides.
    session.say(group_with(
        1,
        "DP-1",
        &[
            (12, "Mail", "mail", 2, false),
            (10, "Main", "main", 0, true),
            (11, "Code", "code", 1, false),
        ],
    ));
    let workspaces = project_workspaces(session.state());
    assert_eq!(workspaces.groups.len(), 1);
    let group = &workspaces.groups[0];
    assert_eq!(group.outputs, [OutputName("DP-1".to_owned())]);
    let names: Vec<(&str, Activity)> = group
        .workspaces
        .iter()
        .map(|w| (w.name.as_str(), w.activity))
        .collect();
    assert_eq!(
        names,
        [
            ("Main", Activity::Active),
            ("Code", Activity::Inactive),
            ("Mail", Activity::Inactive)
        ]
    );
    assert_eq!(group.workspaces[1].stable_id, "code");
    assert!(
        group.workspaces[0]
            .capabilities
            .contains(sill_model::cosmic_wl::WorkspaceCap::Activate),
        "the ability the compositor gave"
    );
    assert!(
        !group.workspaces[0]
            .capabilities
            .contains(sill_model::cosmic_wl::WorkspaceCap::Pin),
        "and none it did not"
    );
}

#[test]
fn a_window_is_on_the_active_workspace_of_its_output() {
    let mut session = Session::new();
    session
        .say(group_with(
            1,
            "DP-1",
            &[(10, "One", "one", 0, true), (11, "Two", "two", 1, false)],
        ))
        .say(group_with(2, "DP-2", &[(20, "Three", "three", 0, true)]))
        .say(opens(1, "term", &[], "DP-1"))
        .say(opens(2, "files", &[], "DP-2"));
    let all = project_toplevels(Toplevels::default(), session.state());
    let on = |key: u64| {
        all.windows
            .iter()
            .find(|w| w.key == WindowKey(key))
            .map(|w| w.workspaces.clone())
            .unwrap_or_default()
    };
    assert_eq!(on(1), [WorkspaceKey(10)]);
    assert_eq!(on(2), [WorkspaceKey(20)]);
}

#[test]
fn the_view_ranks_apps_by_recency_and_names_what_is_active() {
    let mut session = Session::new();
    session
        .say(group_with(
            1,
            "DP-1",
            &[
                (10, "Main", "main", 0, true),
                (11, "Code", "code", 1, false),
            ],
        ))
        .say(opens(1, "term", &[], "DP-1"))
        .say(opens(2, "calc", &[], "DP-1"))
        .say(opens(3, "term", &[Maximized], "DP-1"))
        .say(opens(4, "", &[], "DP-1"));
    let recency = project_toplevels(Toplevels::default(), session.state());
    session.say([flags(2, &[Activated])]);
    let recency = project_toplevels(recency, session.state());
    let view = desktop_view(session.state(), &recency);

    let apps: Vec<(&str, u32)> = view.apps.iter().map(|a| (a.key.as_str(), a.mru)).collect();
    assert_eq!(
        apps,
        [("calc", 0), ("term", 1)],
        "one per app id, an empty id is no app"
    );
    let windows: Vec<&str> = view.windows.iter().map(|w| w.identifier.as_str()).collect();
    assert_eq!(windows, ["id-2", "id-1", "id-3", "id-4"]);
    assert_eq!(view.active.app.as_deref(), Some("calc"));
    assert_eq!(view.active.key_window.as_deref(), Some("id-2"));
    assert!(
        view.windows
            .iter()
            .all(|w| w.space == SpacePlaceView::Unknown),
        "wlr cannot say"
    );
    let spaces: Vec<(String, u32, Shown)> = view
        .spaces
        .iter()
        .map(|s| (s.id.to_string(), s.position, s.shown))
        .collect();
    assert_eq!(
        spaces,
        [
            ("main".to_owned(), 0, Shown::Shown),
            ("code".to_owned(), 1, Shown::Hidden)
        ]
    );
}

#[test]
fn a_space_id_is_the_compositors_id_made_into_a_slug() {
    let cases = [
        ("a plain id", Some("Work"), "work"),
        ("spaces and marks", Some("My Space!"), "my-space-"),
        ("none given", None, "ws-7"),
        ("only marks", Some("..."), "ws-7"),
        ("the reserved word", Some("desktop"), "ws-7"),
    ];
    for (name, id, want) in cases {
        let mut session = Session::new();
        session.say([
            DesktopEvent::Workspace(WorkspaceEvent::Opened(wk(7))),
            DesktopEvent::Workspace(WorkspaceEvent::Done),
        ]);
        if let Some(id) = id {
            session.say([
                DesktopEvent::Workspace(WorkspaceEvent::Changed {
                    key: wk(7),
                    change: WorkspaceChange::Id(id.to_owned()),
                }),
                DesktopEvent::Workspace(WorkspaceEvent::Done),
            ]);
        }
        let workspace = session
            .state()
            .workspaces
            .get(wk(7))
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(space_id(workspace).as_str(), want, "{name}");
    }
}

#[test]
fn the_shells_commands_become_the_wire_requests_the_protocols_carry() {
    let mut session = Session::new();
    session
        .say(group_with(
            1,
            "DP-1",
            &[
                (10, "Main", "main", 0, true),
                (11, "Code", "code", 1, false),
            ],
        ))
        .say(opens(1, "term", &[], "DP-1"))
        .say(opens(2, "files", &[Minimized], "DP-1"));
    let state = session.state();
    let wire = |command: WlCommand| {
        desktop_of(&command, state)
            .ok_or(Refusal::Unsupported)
            .and_then(|command| wire_of(&command, state))
    };
    let cases: Vec<WireCase> = vec![
        (
            "activate",
            WlCommand::Activate(WindowKey(1)),
            Ok(vec![Wire::Activate(tk(1))]),
        ),
        (
            "activate restores a minimised window first",
            WlCommand::Activate(WindowKey(2)),
            Ok(vec![Wire::Unminimise(tk(2)), Wire::Activate(tk(2))]),
        ),
        (
            "close",
            WlCommand::Close(WindowKey(1)),
            Ok(vec![Wire::Close(tk(1))]),
        ),
        (
            "minimise",
            WlCommand::Minimize(WindowKey(1)),
            Ok(vec![Wire::Minimise(tk(1))]),
        ),
        (
            "unminimise",
            WlCommand::Unminimize(WindowKey(2)),
            Ok(vec![Wire::Unminimise(tk(2))]),
        ),
        (
            "show a workspace",
            WlCommand::ActivateWorkspace(WorkspaceKey(11)),
            Ok(vec![
                Wire::ActivateWorkspace(wk(11)),
                Wire::CommitWorkspaces,
            ]),
        ),
        (
            "a window that is gone",
            WlCommand::Close(WindowKey(99)),
            Err(Refusal::Unsupported),
        ),
        (
            "pinning is COSMIC's",
            WlCommand::PinWorkspace(WorkspaceKey(10)),
            Err(Refusal::Unsupported),
        ),
    ];
    for (name, command, want) in cases {
        assert_eq!(wire(command), want, "{name}");
    }
}

#[test]
fn what_sill_says_a_backend_supports_is_what_the_wire_client_accepts() {
    let mut session = Session::new();
    session
        .say(group_with(
            1,
            "DP-1",
            &[
                (10, "Main", "main", 0, true),
                (11, "Code", "code", 1, false),
            ],
        ))
        .say(opens(1, "term", &[], "DP-1"));
    let state = session.state().clone();
    let place = DesktopCommand::Place("id-1".to_owned(), PlaceTarget::Left);
    let commands = [
        DesktopCommand::Activate("id-1".to_owned()),
        DesktopCommand::Close("id-1".to_owned()),
        DesktopCommand::Minimise("id-1".to_owned()),
        DesktopCommand::Unminimise("id-1".to_owned()),
        DesktopCommand::SwitchSpace(space_id(
            state
                .workspaces
                .get(wk(11))
                .unwrap_or_else(|| panic!("workspace")),
        )),
        place,
        DesktopCommand::ToSpace(
            "id-1".to_owned(),
            space_id(
                state
                    .workspaces
                    .get(wk(11))
                    .unwrap_or_else(|| panic!("workspace")),
            ),
        ),
        DesktopCommand::SetMinimiseTarget {
            toplevel: "id-1".to_owned(),
            rect: (0, 0, 64, 64),
        },
    ];
    for command in &commands {
        let sent = wire_of(command, &state).map(|requests| {
            requests
                .into_iter()
                .try_for_each(|request| session.desktop.send(request).map(drop))
        });
        match supports(DesktopBackend::Wlr, command) {
            Supports::Yes => assert_eq!(sent, Ok(Ok(())), "{command:?} is offered and sent"),
            Supports::No => assert_eq!(sent, Err(Refusal::Unsupported), "{command:?} is refused"),
        }
    }
}

#[test]
fn the_offered_protocols_choose_the_arm() {
    let named = |names: &[&str]| GlobalOffer::from_globals(names.iter().map(|name| (*name, 1)));
    let list_and_workspaces = ["ext_foreign_toplevel_list_v1", "ext_workspace_manager_v1"];
    let cases = [
        (
            "all four",
            offer_of_protocols(&full_offer()),
            DesktopBackend::Wlr,
        ),
        (
            "the list and workspaces alone",
            named(&list_and_workspaces),
            DesktopBackend::Wlr,
        ),
        (
            "the list beside COSMIC's own toplevel info",
            named(&[
                list_and_workspaces[0],
                list_and_workspaces[1],
                "zcosmic_toplevel_info_v1",
            ]),
            DesktopBackend::Cosmic,
        ),
        (
            "the desktop's own manager",
            offer_of_protocols(&full_offer().with(Protocol::QuireDesktop)),
            DesktopBackend::Quire,
        ),
        (
            "the desktop's own manager without the list",
            named(&["quire_desktop_manager_v1"]),
            DesktopBackend::Cosmic,
        ),
        ("nothing", GlobalOffer::default(), DesktopBackend::Cosmic),
    ];
    for (name, offer, want) in cases {
        assert_eq!(backend_of(&offer), want, "{name}");
    }
}

#[test]
fn keycap_is_told_the_window_the_compositor_calls_activated() {
    let mut session = Session::new();
    session
        .say(opens(1, "term", &[], "DP-1"))
        .say(opens(2, "calc", &[], "DP-1"));
    let before = project_toplevels(Toplevels::default(), session.state());
    let index = AppIndex::default();
    assert!(
        focused(&before, &index).is_none(),
        "nothing activated, nothing focused"
    );
    session.say([flags(2, &[Activated])]);
    let after = project_toplevels(before, session.state());
    assert_eq!(
        focused(&after, &index).map(|f| f.toplevel.app_id.clone()),
        Some("calc".to_owned())
    );
}

/// One head, `DP-1`, with two modes, at scale 1.5.
fn one_head(session: &mut Session) {
    let (head, small, big) = (HeadKey::new(1), ModeKey::new(2), ModeKey::new(3));
    let head_change = |change| DesktopEvent::Output(OutputEvent::Head { key: head, change });
    let mode = |mode, change| DesktopEvent::Output(OutputEvent::Mode { mode, change });
    session.say([
        DesktopEvent::Output(OutputEvent::HeadOpened(head)),
        head_change(HeadChange::Name(output("DP-1"))),
        head_change(HeadChange::Make("BenQ".to_owned())),
        head_change(HeadChange::Model("MA270U".to_owned())),
        head_change(HeadChange::PhysicalSize(Size::new(597, 336))),
        head_change(HeadChange::Enabled(Enabled::On)),
        DesktopEvent::Output(OutputEvent::ModeOpened { head, mode: small }),
        mode(small, ModeChange::Size(Size::new(2560, 1440))),
        mode(small, ModeChange::Refresh(RefreshMilliHz::new(59951))),
        DesktopEvent::Output(OutputEvent::ModeOpened { head, mode: big }),
        mode(big, ModeChange::Size(Size::new(3840, 2160))),
        mode(big, ModeChange::Refresh(RefreshMilliHz::new(59997))),
        head_change(HeadChange::CurrentMode(big)),
        head_change(HeadChange::Position(Point::new(0, 0))),
        head_change(HeadChange::Scale(ScaleMilli::new(1500))),
        DesktopEvent::Output(OutputEvent::Done { serial: 4 }),
    ]);
}

#[test]
fn heads_are_described_as_the_displays_read_them() {
    let mut session = Session::new();
    one_head(&mut session);
    let heads = heads_of(&session.state().outputs);
    assert_eq!(heads.len(), 1);
    let head = &heads[0];
    assert_eq!(head.name, OutputName("DP-1".to_owned()));
    assert_eq!(head.enablement, Enablement::Enabled);
    assert_eq!(
        (head.make.as_str(), head.model.as_str()),
        ("BenQ", "MA270U")
    );
    assert_eq!(head.physical.map(|p| (p.width, p.height)), Some((597, 336)));
    assert_eq!(
        head.current
            .map(|m| (m.pixels.width, m.pixels.height, m.refresh_mhz)),
        Some((3840, 2160, 59997))
    );
    assert_eq!(head.scale, DisplayScale(150));
}

#[test]
fn a_scale_becomes_a_configuration_that_keeps_the_rest() {
    let mut session = Session::new();
    one_head(&mut session);
    let outputs = &session.state().outputs;
    let name = OutputName("DP-1".to_owned());
    let config =
        config_for_scale(outputs, &name, DisplayScale(200)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(config.heads.len(), 1);
    match &config.heads[0].request {
        HeadChangeRequest::Enable(settings) => {
            assert_eq!(settings.scale, Some(ScaleMilli::new(2000)));
            assert_eq!(
                (&settings.mode, settings.position, settings.transform),
                (&None, None, None)
            );
        }
        other => panic!("{other:?}"),
    }
    let unknown = config_for_scale(outputs, &OutputName("HDMI-9".to_owned()), DisplayScale(100));
    assert_eq!(unknown, Err(ScaleRefusal::UnknownHead("HDMI-9".to_owned())));
}
