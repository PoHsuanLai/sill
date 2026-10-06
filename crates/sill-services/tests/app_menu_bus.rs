//! The app-menu service and the agent's two menu actions against `sill-testkit`'s fake app on a
//! private bus, never the real session bus. The fake app exports a menu bar as quire apps do
//! (`org.quire.AppMenu.<app_id>`, `com.canonical.dbusmenu`) and logs every call it receives.
//! The service follows the app's menu, a click reaches the app as `Event "clicked"`, a menu
//! changed under it is followed, and an app with no owner has no entry. The agent lists the menu
//! and activates items by the effect table: a verified app's own command goes through intentd
//! (a recording transport) and never as a click; everything else the app sends is destructive.
//! The tests need a `dbus-daemon` and skip, saying so, without one.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use docket_client::{Intents, Transport, TransportError};
use docket_core::Follow;
use docket_core::{
    CallRequest, IntentsReply, IntentsRequest, Manifest, Origin, Outcome, Preview, Undoable,
    ValidManifest, validate,
};
use prov::Effect;
use sill_launcher::WindowKey;
use sill_model::app_menu::{
    AppMenuCommand, AppMenus, Follower, MenuApp, MenuName, ServedMenu, titles,
};
use sill_model::cosmic_wl::{
    Activity, CosmicWl, Sizing, Toplevel, Toplevels, WindowState, WlCommand,
};
use sill_model::ids::OutputName;
use sill_services::app_menu::{
    Activated, AppMenuDeps, AppMenuService, MenuAgent, MenuFault, MenuLinks,
};
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::{Handle, spawn};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::Connection;

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
    fn start(bus: &PrivateBus, scratch: &Path, app_id: &str) -> FakeApp {
        use std::io::{BufRead, BufReader};
        let log = scratch.join(format!("{app_id}.log"));
        let mut child = Command::new(app_menu_app())
            .arg(&bus.address)
            .arg(&log)
            .arg(name_of(&MenuApp(app_id.to_owned())).map_or_else(String::new, |n| n.0))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("starting app_menu_app: {e}"));
        let stdout = child.stdout.take().unwrap_or_else(|| panic!("no stdout"));
        let mut line = String::new();
        BufReader::new(stdout)
            .read_line(&mut line)
            .unwrap_or_else(|e| panic!("{e}"));
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

/// The service on `bus`, in this process.
fn service(bus: &PrivateBus, scratch: &Path) -> Handle<AppMenuService> {
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(scratch)
    };
    spawn::<AppMenuService>(
        &tokio::runtime::Handle::current(),
        &env,
        AppMenuDeps { name_of },
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
    let app = FakeApp::start(&bus, scratch.path(), "test.app");
    let handle = service(&bus, scratch.path());
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
                vec!["Sidebar".to_owned(), "Erase All".to_owned()]
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
    let handle = service(&bus, scratch.path());
    let late = MenuApp("test.app".to_owned());

    follow(&handle, Some("test.app"));
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(
        *handle.state.borrow(),
        AppMenus::default(),
        "no owner, no menu"
    );

    let app = FakeApp::start(&bus, scratch.path(), "test.app");
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
    _windows: watch::Sender<CosmicWl>,
}

async fn agent(
    bus: &PrivateBus,
    scratch: &Path,
    apps: &[&str],
    proof: Option<(u32, &str)>,
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
    let windows = CosmicWl {
        toplevels: Toplevels {
            windows: apps
                .iter()
                .zip(1..)
                .map(|(app, key)| window(key, app))
                .collect(),
        },
        ..CosmicWl::default()
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
    let _app = FakeApp::start(&bus, scratch.path(), "test.app");
    let agent = agent(&bus, scratch.path(), &["test.app", "other"], None).await;

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
    let app = FakeApp::start(&bus, scratch.path(), "test.app");
    let mut agent = agent(&bus, scratch.path(), &["test.app"], None).await;

    // An item that names another app's harmless Read action: a claim, so a destructive click.
    let spoof = agent.agent.activate(None, "22", Origin::Companion).await;
    let spoof = spoof.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (spoof.how, spoof.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    app.logged("Event 22 clicked").await;

    // An item under a View title: the title is the app's too.
    let erase = agent.agent.activate(None, "42", Origin::Companion).await;
    let erase = erase.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (erase.how, erase.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    app.logged("Event 42 clicked").await;

    // The command id of its own manifest's action, with nobody proving the app: still a click.
    let own = agent.agent.activate(None, "21", Origin::Companion).await;
    let own = own.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (own.how, own.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    assert!(agent.performed().is_empty(), "nothing went through intentd");

    // A submenu parent and an unknown key run nothing.
    assert_eq!(
        agent
            .agent
            .activate(None, "20", Origin::Companion)
            .await
            .map(|a| a.how),
        Err(MenuFault::NotRunnable)
    );
    assert_eq!(
        agent
            .agent
            .activate(None, "9999", Origin::Companion)
            .await
            .map(|a| a.how),
        Err(MenuFault::NoSuchItem)
    );
    assert!(
        agent.wl.try_recv().is_err(),
        "the compositor was never asked"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_verified_apps_own_command_goes_through_intentd_and_never_as_a_click() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-verified");
    let app = FakeApp::start(&bus, scratch.path(), "dev.notes");
    let agent = agent(
        &bus,
        scratch.path(),
        &["dev.notes"],
        Some((app.pid, "dev.notes")),
    )
    .await;

    let done = agent.agent.activate(None, "21", Origin::Companion).await;
    let done = done.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        done.effect,
        Effect::UndoableWrite,
        "the action's own declared effect"
    );
    let performed = agent.performed();
    assert_eq!(performed.len(), 1, "{performed:?}");
    assert_eq!(performed[0].action.app.as_str(), "dev.notes");
    assert_eq!(performed[0].action.name.as_str(), "notes.archive");
    assert_eq!(performed[0].origin, Origin::Companion);
    assert!(
        !app.lines().iter().any(|l| l.starts_with("Event ")),
        "no dbusmenu click was sent: {:?}",
        app.lines()
    );

    // A command this app's manifest does not declare is a claim, even from the verified app.
    let spoof = agent.agent.activate(None, "22", Origin::Companion).await;
    let spoof = spoof.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (spoof.how, spoof.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    assert_eq!(agent.performed().len(), 1);
    app.logged("Event 22 clicked").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_name_owned_by_another_app_than_it_names_is_not_verified() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-impostor");
    // The process serves dev.notes's menu name but was launched as dev.other.
    let app = FakeApp::start(&bus, scratch.path(), "dev.notes");
    let agent = agent(
        &bus,
        scratch.path(),
        &["dev.notes"],
        Some((app.pid, "dev.other")),
    )
    .await;

    let done = agent.agent.activate(None, "21", Origin::Companion).await;
    let done = done.unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        (done.how, done.effect),
        (Activated::Clicked, Effect::Destructive)
    );
    assert!(agent.performed().is_empty());
    app.logged("Event 21 clicked").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shells_own_rows_do_what_the_table_calls_cheap() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("app-menu-own");
    let mut agent = agent(&bus, scratch.path(), &["foot", "files"], None).await;

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
        let done = agent.agent.activate(None, item, Origin::Companion).await;
        let done = done.unwrap_or_else(|e| panic!("{item}: {e:?}"));
        assert_eq!((done.how, done.effect), (Activated::Own, effect), "{item}");
        let mut got = Vec::new();
        while let Ok(command) = agent.wl.try_recv() {
            got.push(command);
        }
        assert_eq!(got, want, "{item}");
    }
}
