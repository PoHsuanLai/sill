//! The app-menu service and the agent's two menu actions against `sill-testkit`'s fake app on a
//! private bus, never the real session bus. The fake app exports a menu bar as quire apps do
//! (`org.quire.AppMenu.<app_id>`, `com.canonical.dbusmenu`) and logs every call it receives.
//! The service follows the app's menu, a click reaches the app as `Event "clicked"`, a menu
//! changed under it is followed, and an app with no owner has no entry. The agent lists the menu
//! and activates items by the effect table: a verified app's own command is handed back as the next call
//! (never done here, never a click); everything else the app sends is destructive.
//! The tests need a `dbus-daemon` and skip, saying so, without one.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use docket_client::{Intents, Transport, TransportError};
use docket_core::Follow;
use docket_core::{
    ActionRef, CallClass, CallRequest, IntentsReply, IntentsRequest, Manifest, Origin, Outcome,
    Preview, TargetValue, Undoable, ValidManifest, validate,
};
use prov::{ActionName, AppName, Effect};
use sill_launcher::WindowKey;
use sill_model::app_menu::{
    AppMenuCommand, AppMenus, Follower, MenuApp, MenuName, ServedMenu, titles,
};
use sill_model::ids::OutputName;
use sill_model::windows::{
    Activity, Sizing, Subjects, Toplevel, Toplevels, WindowModel, WindowState, WlCommand,
};
use sill_services::app_menu::{
    Activated, AppMenuDeps, AppMenuService, MenuAgent, MenuFault, MenuLinks,
};
use sill_services::callers::CallerSource;
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::Connection;

use crate::callers_support::Introduced;

const WAIT: Duration = Duration::from_secs(20);

/// The stand-in app (`src/bin/app_menu_app.rs`), built with this test: nextest's own path when it
/// runs from an archive, else the one cargo gave this build. Nothing here shells out to cargo.
fn app_menu_app() -> PathBuf {
    std::env::var_os("NEXTEST_BIN_EXE_app_menu_app").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_app_menu_app")),
        PathBuf::from,
    )
}

/// The bus name of `app`'s menu: these ids are already legal bus-name elements, so quire's
/// derivation (`sill_ui_kit::app_menu::menu_name_of`, held to it by its own test) is the prefix and
/// the id.
fn name_of(app: &MenuApp) -> Option<MenuName> {
    Some(MenuName(format!("org.quire.AppMenu.{}", app.0)))
}

/// A fake app serving the menu of `app_id`, and its log.
struct FakeApp {
    child: Child,
    log: PathBuf,
    pid: u32,
}

impl FakeApp {
    /// The app `app_id`, which the system shows to have been started as `app_id`.
    fn start(bus: &PrivateBus, scratch: &Path, app_id: &str, callers: &Introduced) -> FakeApp {
        FakeApp::start_as(bus, scratch, app_id, callers, app_id)
    }

    /// The app `app_id`, which the system shows to have been started as `launched_as`. The app
    /// connects first and takes its menu name only after the test has said who the connection is,
    /// so the service never looks at the name before it can know the owner.
    fn start_as(
        bus: &PrivateBus,
        scratch: &Path,
        app_id: &str,
        callers: &Introduced,
        launched_as: &str,
    ) -> FakeApp {
        use std::io::{BufRead, BufReader, Write};
        let log = scratch.join(format!("{app_id}.log"));
        let mut child = Command::new(app_menu_app())
            .arg(&bus.address)
            .arg(&log)
            .arg(name_of(&MenuApp(app_id.to_owned())).map_or_else(String::new, |n| n.0))
            .arg("--handshake")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("starting app_menu_app: {e}"));
        let stdout = child.stdout.take().unwrap_or_else(|| panic!("no stdout"));
        let mut stdin = child.stdin.take().unwrap_or_else(|| panic!("no stdin"));
        let mut lines = BufReader::new(stdout);
        let mut line = String::new();
        lines.read_line(&mut line).unwrap_or_else(|e| panic!("{e}"));
        let unique = line
            .trim()
            .strip_prefix("app_menu_app: connected ")
            .unwrap_or_else(|| panic!("{line:?}"))
            .to_owned();
        callers.launched_as(&unique, "/usr/bin/app_menu_app", launched_as);
        stdin.write_all(b"go\n").unwrap_or_else(|e| panic!("{e}"));
        line.clear();
        lines.read_line(&mut line).unwrap_or_else(|e| panic!("{e}"));
        assert!(line.starts_with("app_menu_app: serving"), "{line:?}");
        let pid = child.id();
        FakeApp { child, log, pid }
    }

    fn lines(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The log once it has `line`, within [`WAIT`].
    async fn logged(&self, line: &str) -> Vec<String> {
        let end = Instant::now() + WAIT;
        loop {
            let lines = self.lines();
            if lines.iter().any(|l| l == line) {
                return lines;
            }
            assert!(
                Instant::now() < end,
                "the app never logged {line:?}: {lines:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Drop for FakeApp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The service on `bus`, in this process, whose windows say nothing of any program.
fn service(bus: &PrivateBus, scratch: &Path, callers: &Arc<Introduced>) -> Handle<AppMenuService> {
    let (_windows, receiver) = watch::channel(WindowModel::default());
    service_seeing(bus, scratch, callers, receiver)
}

/// The service on `bus`, in this process, reading the compositor's windows from `windows`.
fn service_seeing(
    bus: &PrivateBus,
    scratch: &Path,
    callers: &Arc<Introduced>,
    windows: watch::Receiver<WindowModel>,
) -> Handle<AppMenuService> {
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        callers: callers.clone(),
        ..Env::isolated(scratch)
    };
    spawn::<AppMenuService>(
        &tokio::runtime::Handle::current(),
        &env,
        AppMenuDeps { name_of, windows },
    )
}

fn follow(handle: &Handle<AppMenuService>, app: Option<&str>) {
    let sent = handle.control.send(AppMenuCommand::Follow {
        by: Follower("DP-1".to_owned()),
        app: app.map(|app| MenuApp(app.to_owned())),
    });
    assert!(sent.is_ok(), "the service is there");
}

/// The published menus once `ok`, within [`WAIT`].
async fn menus_where(
    handle: &Handle<AppMenuService>,
    what: &str,
    ok: impl Fn(&AppMenus) -> bool,
) -> AppMenus {
    let mut state = handle.state.clone();
    let end = Instant::now() + WAIT;
    loop {
        let now = state.borrow_and_update().clone();
        if ok(&now) {
            return now;
        }
        let left = end
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| panic!("timed out waiting for {what}; last {now:?}"));
        let _ = tokio::time::timeout(left, state.changed()).await;
    }
}

/// `(title, row labels)` of each of the app's titles after its name, and the first menu's labels.
fn outline(served: &ServedMenu) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let made = titles(served);
    let labels = |rows: &[sill_model::tray::TrayMenuEntry]| {
        rows.iter()
            .filter_map(|row| match &row.kind {
                sill_model::tray::TrayMenuKind::Item { label, .. } => Some(label.clone()),
                sill_model::tray::TrayMenuKind::Separator => None,
            })
            .collect::<Vec<_>>()
    };
    (
        made.app.map(|(_, rows)| labels(&rows)).unwrap_or_default(),
        made.menus
            .iter()
            .map(|menu| (menu.title.clone(), labels(&menu.rows)))
            .collect(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_followed_apps_titles_clicks_and_changes_reach_the_bar_and_back() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-follow");
    let callers = Introduced::new();
    let app = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    let handle = service(&bus, scratch.path(), &callers);
    let test_app = MenuApp("test.app".to_owned());

    follow(&handle, Some("test.app"));
    let menus = menus_where(&handle, "test.app's menu", |m| {
        m.served.contains_key(&test_app)
    })
    .await;
    let served = &menus.served[&test_app];
    let (own, titled) = outline(served);
    assert_eq!(own, ["About Notes", "Quit Notes"]);
    assert_eq!(
        titled,
        [
            (
                "Edit".to_owned(),
                vec!["Undo".to_owned(), "Spoof".to_owned(), "Refresh".to_owned()]
            ),
            (
                "View".to_owned(),
                vec![
                    "Sidebar".to_owned(),
                    "Erase All".to_owned(),
                    "Erase Notes".to_owned()
                ]
            ),
        ]
    );

    // A menu opening is told to the app, a pick reaches it as a click.
    let sent = handle.control.send(AppMenuCommand::Opening {
        app: test_app.clone(),
        id: 20,
    });
    assert!(sent.is_ok());
    app.logged("AboutToShow 20").await;
    let sent = handle.control.send(AppMenuCommand::Pick {
        app: test_app.clone(),
        id: 21,
    });
    assert!(sent.is_ok());
    app.logged("Event 21 clicked").await;

    // Refresh changes the Edit menu under the open popup: LayoutUpdated is followed.
    let sent = handle.control.send(AppMenuCommand::Pick {
        app: test_app.clone(),
        id: 23,
    });
    assert!(sent.is_ok());
    let menus = menus_where(&handle, "Redo in Edit", |m| {
        m.served
            .get(&test_app)
            .is_some_and(|served| outline(served).1[0].1.contains(&"Redo".to_owned()))
    })
    .await;
    assert_eq!(outline(&menus.served[&test_app]).1[0].1.len(), 4);

    // Nobody naming the app any more lets it go.
    follow(&handle, None);
    menus_where(&handle, "the app let go", |m| m.served.is_empty()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_with_no_owner_has_no_menu_until_one_appears_and_none_after_it_leaves() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-owner");
    let callers = Introduced::new();
    let handle = service(&bus, scratch.path(), &callers);
    let late = MenuApp("test.app".to_owned());

    follow(&handle, Some("test.app"));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(
        *handle.state.borrow(),
        AppMenus::default(),
        "no owner, no menu"
    );

    let app = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    menus_where(&handle, "the menu once its app is there", |m| {
        m.served.contains_key(&late)
    })
    .await;
    drop(app);
    menus_where(&handle, "the menu gone with its app", |m| {
        m.served.is_empty()
    })
    .await;
}

/// What the recording transport saw of intentd.
#[derive(Debug, Default)]
struct Seen {
    performed: Vec<CallRequest>,
}

/// A stand-in for intentd: the notes app's manifest, and a record of every call performed.
#[derive(Debug, Clone)]
struct RecordingIntentd {
    seen: Arc<Mutex<Seen>>,
}

const NOTES: &str = r#"
vocab = 1
app = "dev.notes"
entities = []

[[actions]]
name = "notes.archive"
label = "Archive"
on = { kind = "nothing" }
effect = "undoable_write"
classes = ["app_own"]
undo = "token"
reach = "offered"
latency = "quick"
result = { kind = "nothing" }
keys = { kind = "none" }
lasting = "no"
dry_run = "none"
params = []

[[actions]]
name = "notes.erase"
label = "Erase"
on = { kind = "nothing" }
effect = "destructive"
classes = ["app_own"]
undo = "not_undoable"
reach = "offered"
latency = "quick"
result = { kind = "nothing" }
keys = { kind = "none" }
lasting = "no"
dry_run = "none"
params = []
"#;

fn manifests() -> Vec<ValidManifest> {
    let shell = sill_services::intents::manifest().unwrap_or_else(|e| panic!("{e}"));
    let notes: Manifest = toml::from_str(NOTES).unwrap_or_else(|e| panic!("{e}"));
    vec![shell, validate(notes).unwrap_or_else(|e| panic!("{e:?}"))]
}

impl Transport for RecordingIntentd {
    async fn call(&self, request: IntentsRequest) -> Result<IntentsReply, TransportError> {
        match request {
            IntentsRequest::Manifests => Ok(IntentsReply::Manifests(manifests())),
            IntentsRequest::Perform { call, .. } => {
                self.seen
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .performed
                    .push(call);
                Ok(IntentsReply::Performed(Box::new(Ok(Outcome {
                    value: None,
                    said: None,
                    show: Preview::None,
                    undo: Undoable::No,
                    follow: Follow::Nothing,
                }))))
            }
            other => Err(TransportError::Malformed(format!("{other:?}"))),
        }
    }
}

fn window(key: u64, app_id: &str) -> Toplevel {
    Toplevel {
        key: WindowKey(key),
        app_id: app_id.to_owned(),
        title: format!("{app_id} {key}"),
        activity: Activity::Inactive,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: Vec::new(),
        outputs: vec![OutputName("DP-1".to_owned())],
    }
}

/// An agent on `bus` whose windows are `apps` (the first focused), whose intentd records, and
/// whose `/proc` says the process `pid` was launched as `launched_as` (nobody, when `None`).
struct Agent {
    agent: MenuAgent<RecordingIntentd>,
    intentd: RecordingIntentd,
    wl: tokio::sync::mpsc::UnboundedReceiver<WlCommand>,
    _windows: watch::Sender<WindowModel>,
}

async fn agent(
    bus: &PrivateBus,
    scratch: &Path,
    callers: &Arc<Introduced>,
    apps: &[&str],
    proof: Option<(u32, &str)>,
) -> Agent {
    agent_reporting(bus, scratch, callers, apps, proof, Subjects::default()).await
}

/// As [`agent`], whose windows also say which program is behind each app (`subjects`).
async fn agent_reporting(
    bus: &PrivateBus,
    scratch: &Path,
    callers: &Arc<Introduced>,
    apps: &[&str],
    proof: Option<(u32, &str)>,
    subjects: Subjects,
) -> Agent {
    let connection = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let proc_root = scratch.join("proc");
    if let Some((pid, app)) = proof {
        let dir = proc_root.join(pid.to_string());
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let scope = format!("0::/user.slice/user@1000.service/app.slice/app-{app}-4242.scope\n");
        std::fs::write(dir.join("cgroup"), scope).unwrap_or_else(|e| panic!("{e}"));
    }
    let windows = WindowModel {
        toplevels: Toplevels {
            windows: apps
                .iter()
                .zip(1..)
                .map(|(app, key)| window(key, app))
                .collect(),
        },
        subjects,
        ..WindowModel::default()
    };
    let (windows_tx, toplevels) = watch::channel(windows);
    let (wl, wl_commands) = Control::channel();
    let links = MenuLinks {
        toplevels,
        wl,
        name_of,
    };
    let intentd = RecordingIntentd {
        seen: Arc::default(),
    };
    let agent = MenuAgent::with_intents(
        Connection::clone(&connection),
        links,
        proc_root,
        Intents::over(intentd.clone()),
        callers.peers_on(connection.clone()),
    );
    Agent {
        agent,
        intentd,
        wl: wl_commands,
        _windows: windows_tx,
    }
}

impl Agent {
    fn performed(&self) -> Vec<CallRequest> {
        self.intentd
            .seen
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .performed
            .clone()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_listing_names_every_row_and_carries_the_apps_commands() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-list");
    let callers = Introduced::new();
    let _app = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    let agent = agent(&bus, scratch.path(), &callers, &["test.app", "other"], None).await;

    let listed = agent
        .agent
        .list(None)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(listed.app, MenuApp("test.app".to_owned()));
    let text = listed.text;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "test.app");
    for want in [
        "  [12] About Notes command=app.about",
        "  [raise:1] test.app 1",
        "  [minimise:1] Minimise",
        "  [hide-others] Hide Others",
        "Edit",
        "  [21] Undo keys=Super+Z command=notes.archive",
        "  [22] Spoof command=shell.workspace.switch",
        "View",
        "  [41] Sidebar (checked)",
    ] {
        assert!(lines.contains(&want), "{want:?} not in\n{text}");
    }
    assert!(
        !text.contains("[quit]"),
        "the app has its own Quit, so the shell offers none:\n{text}"
    );

    // A window with no menu has the shell's own rows alone, Quit included.
    let bare = agent
        .agent
        .list(Some("other"))
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(bare.text.contains("[quit] Quit other"), "{}", bare.text);
    assert!(!bare.text.contains("Edit"), "{}", bare.text);
    assert_eq!(
        agent.agent.list(Some("nothing.here")).await.map(|l| l.text),
        Err(MenuFault::NoWindow)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unverified_apps_items_are_destructive_whatever_their_titles_or_commands_say() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-spoof");
    let callers = Introduced::new();
    let app = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    let mut agent = agent(&bus, scratch.path(), &callers, &["test.app"], None).await;

    // An item that names another app's harmless Read action: a claim, so a destructive click.
    let spoof = agent.agent.activate(None, "22", None).await;
    let spoof = spoof.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (spoof.how, spoof.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    app.logged("Event 22 clicked").await;

    // An item under a View title: the title is the app's too.
    let erase = agent.agent.activate(None, "42", None).await;
    let erase = erase.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (erase.how, erase.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    app.logged("Event 42 clicked").await;

    // The command id of its own manifest's action, with nobody proving the app: still a click.
    let own = agent.agent.activate(None, "21", None).await;
    let own = own.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (own.how, own.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    assert!(agent.performed().is_empty(), "nothing went through intentd");

    // A submenu parent and an unknown key run nothing.
    assert_eq!(
        agent.agent.activate(None, "20", None).await.map(|a| a.how),
        Err(MenuFault::NotRunnable)
    );
    assert_eq!(
        agent
            .agent
            .activate(None, "9999", None)
            .await
            .map(|a| a.how),
        Err(MenuFault::NoSuchItem)
    );
    assert!(
        agent.wl.try_recv().is_err(),
        "the compositor was never asked"
    );
}

fn notes(name: &str) -> ActionRef {
    ActionRef {
        app: AppName::parse("dev.notes").unwrap_or_else(|e| panic!("{e}")),
        name: ActionName::parse(name).unwrap_or_else(|e| panic!("{e}")),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_verified_apps_own_command_is_handed_back_as_the_next_call_and_never_clicked() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-verified");
    let callers = Introduced::new();
    let app = FakeApp::start(&bus, scratch.path(), "dev.notes", &callers);
    let agent = agent(
        &bus,
        scratch.path(),
        &callers,
        &["dev.notes"],
        Some((app.pid, "dev.notes")),
    )
    .await;

    let done = agent.agent.activate(None, "21", None).await;
    let done = done.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        done.effect,
        Effect::UndoableWrite,
        "the action's own declared effect"
    );
    assert_eq!(
        done.how,
        Activated::Delegated(CallRequest {
            action: notes("notes.archive"),
            target: TargetValue::Nothing,
            args: Default::default(),
            origin: Origin::AppInternal,
        })
    );
    assert!(
        agent.performed().is_empty(),
        "the router makes the call, not the shell"
    );
    assert!(
        !app.lines().iter().any(|l| l.starts_with("Event ")),
        "no dbusmenu click was sent: {:?}",
        app.lines()
    );

    // A command this app's manifest does not declare is a claim, even from the verified app.
    let spoof = agent.agent.activate(None, "22", None).await;
    let spoof = spoof.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (spoof.how, spoof.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    app.logged("Event 22 clicked").await;
}

fn read() -> CallClass {
    CallClass::Effect(Effect::Read)
}

fn destructive() -> CallClass {
    CallClass::Effect(Effect::Destructive)
}

#[tokio::test(flavor = "multi_thread")]
async fn classify_answers_read_destructive_or_delegates_as_the_table_says() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    // Verified: a declared command delegates; a spoof and a View item are destructive.
    let scratch = TempDir::new("app-menu-classify-verified");
    let callers = Introduced::new();
    let app = FakeApp::start(&bus, scratch.path(), "dev.notes", &callers);
    let verified = agent(
        &bus,
        scratch.path(),
        &callers,
        &["dev.notes"],
        Some((app.pid, "dev.notes")),
    )
    .await;
    let cases = [
        ("21", CallClass::Delegates(notes("notes.archive"))),
        ("22", destructive()),
        ("42", destructive()),
        ("hide", read()),
        ("hide-others", read()),
        ("show-all", read()),
        ("minimise:1", read()),
        ("raise:1", read()),
        ("quit", destructive()),
    ];
    for (item, want) in cases {
        let got = verified.agent.classify(None, item).await;
        assert_eq!(got, Ok(want), "{item}");
    }
    assert_eq!(
        verified.agent.classify(None, "20").await,
        Err(MenuFault::NotRunnable)
    );
    assert_eq!(
        verified.agent.classify(None, "9999").await,
        Err(MenuFault::NoSuchItem)
    );

    // Unverified: even the declared command is a claim.
    let scratch = TempDir::new("app-menu-classify-unverified");
    let callers = Introduced::new();
    let _app = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    let unverified = agent(&bus, scratch.path(), &callers, &["test.app"], None).await;
    for item in ["21", "22", "42"] {
        assert_eq!(
            unverified.agent.classify(None, item).await,
            Ok(destructive()),
            "{item}"
        );
    }
    assert_eq!(
        unverified.agent.classify(None, "minimise:1").await,
        Ok(read())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn perform_refuses_an_item_that_changed_since_it_was_gated_and_always_takes_destructive() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-changed");
    let callers = Introduced::new();
    let app = FakeApp::start(&bus, scratch.path(), "dev.notes", &callers);
    let mut agent = agent(
        &bus,
        scratch.path(),
        &callers,
        &["dev.notes"],
        Some((app.pid, "dev.notes")),
    )
    .await;

    // Gated as Read, but item 21 is a delegation now: refused, nothing sent or done.
    let changed = agent.agent.activate(None, "21", Some(&read())).await;
    assert_eq!(changed.map(|a| a.how), Err(MenuFault::Changed));
    // Gated as a delegation of another action: refused.
    let other = CallClass::Delegates(notes("notes.erase"));
    let changed = agent.agent.activate(None, "21", Some(&other)).await;
    assert_eq!(changed.map(|a| a.how), Err(MenuFault::Changed));
    // Gated as Read, but the item is a click now.
    let changed = agent.agent.activate(None, "22", Some(&read())).await;
    assert_eq!(changed.map(|a| a.how), Err(MenuFault::Changed));
    // Gated as a delegation, but the item is gone from the menu.
    let gone = agent
        .agent
        .activate(
            None,
            "9999",
            Some(&CallClass::Delegates(notes("notes.archive"))),
        )
        .await;
    assert_eq!(gone.map(|a| a.how), Err(MenuFault::NoSuchItem));
    assert!(
        !app.lines().iter().any(|l| l.starts_with("Event ")),
        "nothing was clicked: {:?}",
        app.lines()
    );
    assert!(agent.wl.try_recv().is_err());

    // The same item as gated is carried out: the delegation is handed back.
    let ok = agent
        .agent
        .activate(
            None,
            "21",
            Some(&CallClass::Delegates(notes("notes.archive"))),
        )
        .await;
    assert!(matches!(ok.map(|a| a.how), Ok(Activated::Delegated(_))));
    let own = agent
        .agent
        .activate(None, "minimise:1", Some(&read()))
        .await;
    assert_eq!(own.map(|a| a.how), Ok(Activated::Own));
    assert_eq!(agent.wl.try_recv(), Ok(WlCommand::Minimize(WindowKey(1))));

    // Destructive is the ceiling, the router's re-ask path: always accepted, whatever it is now.
    let click = agent.agent.activate(None, "22", Some(&destructive())).await;
    assert_eq!(click.map(|a| a.how), Ok(Activated::Clicked));
    app.logged("Event 22 clicked").await;
    let delegated = agent.agent.activate(None, "21", Some(&destructive())).await;
    assert!(matches!(
        delegated.map(|a| a.how),
        Ok(Activated::Delegated(_))
    ));
    let quit = agent
        .agent
        .activate(None, "quit", Some(&destructive()))
        .await;
    assert_eq!(quit.map(|a| a.how), Ok(Activated::Own));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_menu_served_by_another_program_than_the_apps_is_neither_listed_nor_clicked() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-squatter");
    let callers = Introduced::new();
    // The program that owns dev.notes's menu name is the system's, and was launched as dev.other.
    let squatter = FakeApp::start_as(&bus, scratch.path(), "dev.notes", &callers, "dev.other");
    let agent = agent(&bus, scratch.path(), &callers, &["dev.notes"], None).await;

    let listed = agent
        .agent
        .list(None)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(
        !listed.text.contains("Undo"),
        "the squatter's menu was listed: {}",
        listed.text
    );
    let clicked = agent.agent.activate(None, "21", None).await;
    assert!(matches!(clicked, Err(MenuFault::NoMenu)), "{clicked:?}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !squatter.lines().iter().any(|l| l.starts_with("Event ")),
        "the squatter was clicked: {:?}",
        squatter.lines()
    );
}

/// The subject of `app` that the compositor reports, as the windows service publishes it.
fn reporting(app: &str, subject: &str) -> Subjects {
    Subjects::collect([(app.to_owned(), Some(subject.to_owned()))])
}

/// QUIRE-12: with a subject the compositor established, the program that owns the name is
/// compared with it, not with the scope name its launcher claimed.
#[tokio::test(flavor = "multi_thread")]
async fn the_bar_shows_the_menu_of_the_program_the_compositor_reports_whatever_its_scope_says() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-reported");
    let callers = Introduced::new();
    // Launched under another app's scope name, but it is the image the compositor reports.
    let _app = FakeApp::start_as(&bus, scratch.path(), "test.app", &callers, "dev.other");
    let (_windows, receiver) = watch::channel(WindowModel {
        subjects: reporting("test.app", "/usr/bin/app_menu_app"),
        ..WindowModel::default()
    });
    let handle = service_seeing(&bus, scratch.path(), &callers, receiver);
    follow(&handle, Some("test.app"));
    menus_where(&handle, "the reported program's menu", |m| {
        m.served.contains_key(&MenuApp("test.app".to_owned()))
    })
    .await;
}

/// QUIRE-12: a program that launched under the app's own scope name is still not shown when the
/// compositor says the window belongs to another program.
#[tokio::test(flavor = "multi_thread")]
async fn the_bar_shows_no_menu_for_a_program_other_than_the_one_the_compositor_reports() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-reported-other");
    let callers = Introduced::new();
    let _squatter = FakeApp::start(&bus, scratch.path(), "test.app", &callers);
    let (_windows, receiver) = watch::channel(WindowModel {
        subjects: reporting("test.app", "/usr/bin/notes"),
        ..WindowModel::default()
    });
    let handle = service_seeing(&bus, scratch.path(), &callers, receiver);
    follow(&handle, Some("test.app"));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(
        *handle.state.borrow(),
        AppMenus::default(),
        "a menu from a program the compositor does not report was shown"
    );
}

/// QUIRE-12 for the agent: the same comparison decides what it may list.
#[tokio::test(flavor = "multi_thread")]
async fn the_agent_lists_the_menu_of_the_program_the_compositor_reports_and_no_other() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-agent-reported");
    let callers = Introduced::new();
    let _app = FakeApp::start_as(&bus, scratch.path(), "dev.notes", &callers, "dev.other");
    let reported = agent_reporting(
        &bus,
        scratch.path(),
        &callers,
        &["dev.notes"],
        None,
        reporting("dev.notes", "/usr/bin/app_menu_app"),
    )
    .await;
    let listed = reported
        .agent
        .list(None)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(listed.text.contains("Undo"), "{}", listed.text);

    let other = agent_reporting(
        &bus,
        scratch.path(),
        &callers,
        &["dev.notes"],
        None,
        reporting("dev.notes", "/usr/bin/notes"),
    )
    .await;
    let listed = other
        .agent
        .list(None)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(!listed.text.contains("Undo"), "{}", listed.text);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bar_shows_no_menu_for_a_name_another_program_owns() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-bar-squatter");
    let callers = Introduced::new();
    let _squatter = FakeApp::start_as(&bus, scratch.path(), "test.app", &callers, "dev.other");
    let handle = service(&bus, scratch.path(), &callers);
    follow(&handle, Some("test.app"));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(
        *handle.state.borrow(),
        AppMenus::default(),
        "a menu from a program that is not the app was shown"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shells_own_rows_do_what_the_table_calls_cheap() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-own");
    let callers = Introduced::new();
    let mut agent = agent(&bus, scratch.path(), &callers, &["foot", "files"], None).await;

    let cases = [
        (
            "hide",
            Effect::Read,
            vec![WlCommand::Minimize(WindowKey(1))],
        ),
        (
            "hide-others",
            Effect::Read,
            vec![WlCommand::Minimize(WindowKey(2))],
        ),
        (
            "minimise:1",
            Effect::Read,
            vec![WlCommand::Minimize(WindowKey(1))],
        ),
        (
            "raise:2",
            Effect::Read,
            vec![WlCommand::Activate(WindowKey(2))],
        ),
        ("show-all", Effect::Read, Vec::new()),
        (
            "quit",
            Effect::Destructive,
            vec![WlCommand::Close(WindowKey(1))],
        ),
    ];
    for (item, effect, want) in cases {
        let done = agent.agent.activate(None, item, None).await;
        let done = done.unwrap_or_else(|e| panic!("{item}: {e:?}"));
        assert_eq!((done.how, done.effect), (Activated::Own, effect), "{item}");
        let mut got = Vec::new();
        while let Ok(command) = agent.wl.try_recv() {
            got.push(command);
        }
        assert_eq!(got, want, "{item}");
    }
}

// ---- The shell's menu provider under docket's real router ---------------------------------

mod routed {
    use super::*;
    use docket_core::{
        ActionGrantKey, AppRefusal, CallRefusal, CallerId, CallerRole, ClassifyFault,
        ConfirmAnswer, GrantCaller, GrantScope as ConsentScope, GrantTarget, Invocation, Latency,
        ParamName, SessionOpen, Value,
    };
    use docket_fake::{
        FakeMemory, FixedClock, MemoryGrants, RecordingSink, ScriptedConfirmer, ScriptedReader,
        ScriptedReviewer, ScriptedWriter,
    };
    use docket_router::{AppFault, AppLink, GrantStore, LinkFault, NoLog, Router, Seams};
    use policy_point::Pdp;
    use porter_core::consent::{Decision, Grant, Usage};
    use porter_core::{AppId, DataClass, Isolation};
    use prov::{AgentRef, SpaceId, SpaceScope, UnixSeconds};
    use sill_model::intents::shell::menu_ask_of;
    use sill_services::intents::menus;
    use std::collections::BTreeSet;

    /// The apps of the router: the shell, whose menu provider is the real one over the real
    /// agent, and dev.notes, which records what it is asked to do.
    struct Link {
        shell: Agent,
        notes_done: Mutex<Vec<String>>,
    }

    fn shell_app() -> AppName {
        AppName::parse("org.quire.Shell").unwrap_or_else(|e| panic!("{e}"))
    }

    impl AppLink for Link {
        async fn perform(
            &self,
            app: &AppName,
            inv: Invocation,
            within: Latency,
        ) -> Result<Outcome, AppFault> {
            self.perform_classified(app, inv, None, None, within).await
        }

        async fn perform_classified(
            &self,
            app: &AppName,
            inv: Invocation,
            _activation: Option<docket_core::ActivationToken>,
            classified: Option<CallClass>,
            _within: Latency,
        ) -> Result<Outcome, AppFault> {
            if *app == shell_app() {
                let ask = menu_ask_of(&inv)
                    .map_err(AppFault::Refused)?
                    .ok_or(AppFault::Refused(AppRefusal::Unsupported))?;
                return menus::perform(&self.shell.agent, ask, &inv, classified)
                    .await
                    .map_err(AppFault::Refused);
            }
            self.notes_done
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(inv.action.as_str().to_owned());
            Ok(Outcome {
                value: None,
                said: None,
                show: Preview::None,
                undo: Undoable::No,
                follow: Follow::Nothing,
            })
        }

        async fn classify(
            &self,
            app: &AppName,
            inv: Invocation,
        ) -> Result<CallClass, ClassifyFault> {
            if *app != shell_app() {
                return Err(ClassifyFault::Unsupported);
            }
            let ask = menu_ask_of(&inv)
                .ok()
                .flatten()
                .ok_or(ClassifyFault::Unsupported)?;
            menus::classify(&self.shell.agent, ask)
                .await
                .map_err(|_| ClassifyFault::Refused)
        }

        async fn dry_run(&self, _: &AppName, _: Invocation) -> Result<Preview, AppRefusal> {
            Err(AppRefusal::Unsupported)
        }

        async fn undo(
            &self,
            _: &AppName,
            _: &docket_core::UndoToken,
            _: &prov::Actor,
        ) -> Result<(), docket_core::UndoFault> {
            Err(docket_core::UndoFault::AppUnavailable)
        }

        async fn context(
            &self,
            _: &AppName,
            _: docket_core::ContextScope,
        ) -> Result<docket_core::ContextSnapshot, LinkFault> {
            Err(LinkFault::Unavailable)
        }

        async fn search(
            &self,
            _: &AppName,
            _: &str,
            _: docket_core::Generation,
        ) -> Result<Vec<docket_core::Hit>, LinkFault> {
            Err(LinkFault::Unavailable)
        }

        async fn preview(&self, _: &AppName, _: &prov::EntityId) -> Result<Preview, LinkFault> {
            Err(LinkFault::Unavailable)
        }

        async fn suggest(
            &self,
            _: &AppName,
            _: docket_core::SuggestAsk,
        ) -> Result<Vec<docket_core::EntityRef>, LinkFault> {
            Err(LinkFault::Unavailable)
        }
    }

    struct Fakes {
        link: Link,
        confirmer: ScriptedConfirmer,
        reviewer: ScriptedReviewer,
        grants: MemoryGrants,
        sink: RecordingSink,
        clock: FixedClock,
        memory: FakeMemory,
        writer: ScriptedWriter,
        reader: ScriptedReader,
        log: NoLog,
        checkpoints: docket_checkpoint::MemoryStore,
    }

    impl Seams for Fakes {
        type Link = Link;
        type Confirm = ScriptedConfirmer;
        type Review = ScriptedReviewer;
        type Grants = MemoryGrants;
        type Sink = RecordingSink;
        type Time = FixedClock;
        type Memory = FakeMemory;
        type Writer = ScriptedWriter;
        type Reading = ScriptedReader;
        type Log = NoLog;
        type Checkpoints = docket_checkpoint::MemoryStore;
        fn link(&self) -> &Link {
            &self.link
        }
        fn confirmer(&self) -> &ScriptedConfirmer {
            &self.confirmer
        }
        fn reviewer(&self) -> &ScriptedReviewer {
            &self.reviewer
        }
        fn grants(&self) -> &MemoryGrants {
            &self.grants
        }
        fn sink(&self) -> &RecordingSink {
            &self.sink
        }
        fn clock(&self) -> &FixedClock {
            &self.clock
        }
        fn memory(&self) -> &FakeMemory {
            &self.memory
        }
        fn writer(&self) -> &ScriptedWriter {
            &self.writer
        }
        fn reader(&self) -> &ScriptedReader {
            &self.reader
        }
        fn log(&self) -> &NoLog {
            &self.log
        }
        fn checkpoints(&self) -> &docket_checkpoint::MemoryStore {
            &self.checkpoints
        }
    }

    fn companion() -> CallerId {
        CallerId {
            app: AppId {
                name: AppName::parse("org.quire.Companiond").unwrap_or_else(|e| panic!("{e}")),
                isolation: Isolation::Unsandboxed,
            },
            roles: BTreeSet::from([CallerRole::Companion]),
        }
    }

    fn space() -> SpaceId {
        SpaceId::parse("work").unwrap_or_else(|e| panic!("{e}"))
    }

    fn call(item: &str) -> CallRequest {
        let item_arg = prov::Labelled {
            value: Value::Text(item.into()),
            label: prov::Label::trusted_user(),
        };
        CallRequest {
            action: ActionRef {
                app: shell_app(),
                name: ActionName::parse("shell.menu.activate").unwrap_or_else(|e| panic!("{e}")),
            },
            target: TargetValue::Nothing,
            args: std::iter::once((
                ParamName::parse("item").unwrap_or_else(|e| panic!("{e}")),
                item_arg,
            ))
            .collect(),
            origin: Origin::Companion,
        }
    }

    async fn run(router: &Router<Fakes>, call: CallRequest) -> Result<Outcome, CallRefusal> {
        let reply = router
            .handle(
                &companion(),
                docket_core::IntentsRequest::Perform {
                    activation: None,
                    call,
                    session: None,
                    parent_window: None,
                },
            )
            .await;
        match reply {
            IntentsReply::Performed(result) => *result,
            other => panic!("perform: {other:?}"),
        }
    }

    /// A router with a session, the companion's standing consent for both apps (so a first use
    /// asks nothing), and `answers` yeses for the person.
    async fn router(shell: Agent, answers: usize) -> Router<Fakes> {
        let receipt = prov::ConfirmReceipt {
            id: prov::ConfirmId::parse("c-1").unwrap_or_else(|e| panic!("{e}")),
            input: prov::InputProof::HardwareSeat,
            at: UnixSeconds(1),
            covers: prov::Confidentiality::Secret,
        };
        let yes = ConfirmAnswer::Allowed {
            scope: ConsentScope::Once,
            receipt,
            written: None,
        };
        let seams = Fakes {
            link: Link {
                shell,
                notes_done: Mutex::new(Vec::new()),
            },
            confirmer: ScriptedConfirmer::answering((0..answers).map(|_| yes.clone()).collect()),
            reviewer: ScriptedReviewer::always_allow(),
            grants: MemoryGrants::new(),
            sink: RecordingSink::new(),
            clock: FixedClock::at(UnixSeconds(0)),
            memory: FakeMemory::default(),
            writer: ScriptedWriter::failing(),
            reader: ScriptedReader::default(),
            log: NoLog,
            checkpoints: docket_checkpoint::MemoryStore::default(),
        };
        for (owner, n) in [
            (shell_app(), 0),
            (
                AppName::parse("dev.notes").unwrap_or_else(|e| panic!("{e}")),
                2,
            ),
        ] {
            for (m, usage) in [Usage::Interactive, Usage::Background]
                .into_iter()
                .enumerate()
            {
                seams.grants.record(Grant {
                    id: porter_core::GrantId::parse(&format!("g-{}", n + m))
                        .unwrap_or_else(|e| panic!("{e}")),
                    key: ActionGrantKey {
                        caller: GrantCaller::Companion,
                        owner: owner.clone(),
                        target: GrantTarget::App,
                        class: DataClass::AppOwn,
                        usage,
                        space: SpaceScope::Only(space()),
                    },
                    decision: Decision::Allow,
                    scope: ConsentScope::Always,
                    at: UnixSeconds(0),
                });
            }
        }
        let router = Router::new(
            seams,
            docket_core::AgentConfig::default(),
            Pdp::standard().unwrap_or_else(|e| panic!("{e:?}")),
        );
        {
            let mut state = router.state.lock().unwrap_or_else(|e| e.into_inner());
            for manifest in manifests() {
                state.registry.insert(manifest);
            }
        }
        let opened = router
            .handle(
                &companion(),
                docket_core::IntentsRequest::SessionOpen(SessionOpen {
                    space: space(),
                    agent: AgentRef::Companion,
                    parent: None,
                    cwd: None,
                    started_from: None,
                    external: None,
                    helper: None,
                }),
            )
            .await;
        assert!(
            matches!(opened, IntentsReply::SessionOpened(_)),
            "{opened:?}"
        );
        router
    }

    fn asks(router: &Router<Fakes>) -> usize {
        router.seams.confirmer.requests().len()
    }

    fn done(router: &Router<Fakes>) -> Vec<String> {
        router
            .seams
            .link
            .notes_done
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_read_row_asks_nothing_and_a_verified_delegate_asks_once_for_its_own_action() {
        let Some(bus) = PrivateBus::start() else {
            return;
        };
        let scratch = TempDir::new("app-menu-routed");
        let callers = Introduced::new();
        let app = FakeApp::start(&bus, scratch.path(), "dev.notes", &callers);
        let shell = agent(
            &bus,
            scratch.path(),
            &callers,
            &["dev.notes"],
            Some((app.pid, "dev.notes")),
        )
        .await;
        let router = router(shell, 2).await;

        // The shell's own Read row: no ask, and the compositor is told.
        run(&router, call("minimise:1"))
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(asks(&router), 0);
        assert!(done(&router).is_empty());

        // A verified, declared, undoable command: delegated, gated as the app's own action.
        run(&router, call("21"))
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(done(&router), vec!["notes.archive".to_owned()]);
        assert_eq!(
            asks(&router),
            0,
            "an undoable write of the app's own does not ask"
        );

        // A verified destructive command: one ask, for the inner action; then it runs.
        run(&router, call("43"))
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(asks(&router), 1);
        assert_eq!(
            done(&router),
            vec!["notes.archive".to_owned(), "notes.erase".to_owned()]
        );
        assert!(
            !app.lines().iter().any(|l| l.starts_with("Event ")),
            "never a click: {:?}",
            app.lines()
        );

        // A spoof is a destructive click, asked for as such, then sent.
        run(&router, call("22"))
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(asks(&router), 2);
        app.logged("Event 22 clicked").await;
        assert_eq!(done(&router).len(), 2, "nothing more reached dev.notes");
    }
}
