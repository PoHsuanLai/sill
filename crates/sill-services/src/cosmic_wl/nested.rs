//! Live tests of the thread: against a nested cosmic-comp (`dev/nested.sh`), against the
//! session's own compositor (the degrade path, when it lacks the COSMIC protocols), and against
//! no compositor at all.
//!
//! The nested test launches `$SILL_NESTED_CLIENT` ("<program> <app_id>", default `foot foot`)
//! inside the nested compositor.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use sill_launcher::WindowKey;
use tokio::sync::watch;

use super::{Activity, CosmicWl, CosmicWlService, Toplevels, WindowState, WlCommand, Workspaces};
use crate::control::Control;
use crate::env::{Env, WaylandRoute};

/// The service on the compositor `route` names, its state twice (a workspaces reader and a
/// toplevels reader) and its control. The thread starts at once; the runtime only runs the
/// finished start.
fn spawn(
    route: WaylandRoute,
) -> (
    watch::Receiver<CosmicWl>,
    watch::Receiver<CosmicWl>,
    Control<WlCommand>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    let env = Env {
        wayland: route,
        ..Env::isolated(Path::new("/nonexistent/sill-cosmic-wl"))
    };
    let service = crate::service::spawn::<CosmicWlService>(rt.handle(), &env, ());
    (service.state.clone(), service.state, service.control)
}

const WITHIN: Duration = Duration::from_secs(2);

/// Poll `ready` until it answers or `within` passes.
fn wait_for<T>(within: Duration, mut ready: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + within;
    loop {
        if let Some(value) = ready() {
            return Some(value);
        }
        if Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The scheduler state letter (`S` sleeping, `R` running) of this process's thread `name`,
/// while it exists.
fn thread_state(name: &str) -> Option<char> {
    std::fs::read_dir("/proc/self/task").ok()?.find_map(|task| {
        let dir = task.ok()?.path();
        let comm = std::fs::read_to_string(dir.join("comm")).ok()?;
        if comm.trim_end() != name {
            return None;
        }
        let stat = std::fs::read_to_string(dir.join("stat")).ok()?;
        stat.rsplit_once(") ")?.1.chars().next()
    })
}

/// No compositor at all: the thread parks, commands are accepted and dropped, the channels
/// stay open, and dropping the control ends the thread.
#[test]
fn without_a_compositor_the_thread_parks_until_the_control_goes() {
    let missing = std::env::temp_dir().join("sill-no-such-wayland-socket");
    let (workspaces, toplevels, control) = spawn(WaylandRoute::Socket(missing));
    let parked = wait_for(WITHIN, || {
        (thread_state("cosmic_wl") == Some('S')).then_some(())
    });
    assert!(
        parked.is_some(),
        "the thread sleeps: {:?}",
        thread_state("cosmic_wl")
    );
    assert!(control.send(WlCommand::Close(WindowKey(1))).is_ok());
    assert_eq!(
        workspaces.has_changed().ok(),
        Some(false),
        "workspaces open, never sent"
    );
    assert_eq!(
        toplevels.has_changed().ok(),
        Some(false),
        "toplevels open, never sent"
    );
    drop(control);
    let gone = wait_for(WITHIN, || thread_state("cosmic_wl").is_none().then_some(()));
    assert!(
        gone.is_some(),
        "the thread ends once every Control is dropped"
    );
    assert!(
        workspaces.has_changed().is_err(),
        "and the senders go with it"
    );
}

/// The degrade path on the session's own compositor: run with `WAYLAND_DISPLAY` naming a
/// compositor without the COSMIC protocols.
#[test]
#[ignore = "needs a Wayland session without COSMIC protocols"]
fn without_cosmic_protocols_the_channels_stay_empty_and_the_thread_parks() {
    let (workspaces, toplevels, control) = spawn(WaylandRoute::Environment);
    let parked = wait_for(WITHIN, || {
        (thread_state("cosmic_wl") == Some('S')).then_some(())
    });
    assert!(
        parked.is_some(),
        "the thread sleeps: {:?}",
        thread_state("cosmic_wl")
    );
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        thread_state("cosmic_wl"),
        Some('S'),
        "still asleep, not spinning"
    );
    assert!(control.send(WlCommand::Close(WindowKey(1))).is_ok());
    assert_eq!(workspaces.has_changed().ok(), Some(false));
    assert_eq!(toplevels.has_changed().ok(), Some(false));
    assert_eq!(workspaces.borrow().workspaces, Workspaces::default());
    assert_eq!(toplevels.borrow().toplevels, Toplevels::default());
}

/// The nested compositor: started by this test, or one already running (then left running).
struct Nested {
    socket: PathBuf,
    display: String,
    started: Started,
    script: PathBuf,
}

enum Started {
    Here,
    Elsewhere,
}

impl Nested {
    fn start() -> Nested {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let script = repo.join("dev/nested.sh");
        let host_dev = std::env::var_os("SHELL_HOST_DEV")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("../shell-host/scripts/dev"));
        let out = Command::new(&script)
            .arg("start")
            .env("SHELL_HOST_DEV", &host_dev)
            .output()
            .expect("dev/nested.sh runs");
        let started = if out.status.success() {
            Started::Here
        } else {
            Started::Elsewhere
        };
        let env = std::fs::read_to_string(host_dev.join(".nested-env")).unwrap_or_else(|_| {
            panic!(
                "nested.sh start failed and left no .nested-env:\n{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        });
        let display = env
            .lines()
            .find_map(|l| l.strip_prefix("WAYLAND_DISPLAY="))
            .expect(".nested-env names WAYLAND_DISPLAY")
            .to_owned();
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is set");
        Nested {
            socket: PathBuf::from(runtime).join(&display),
            display,
            started,
            script,
        }
    }
}

impl Drop for Nested {
    fn drop(&mut self) {
        if let Started::Here = self.started {
            let _stopped = Command::new(&self.script).arg("stop").status();
        }
    }
}

/// A client inside the nested compositor, killed when dropped.
struct Client(Child);

impl Drop for Client {
    fn drop(&mut self) {
        let _killed = self.0.kill();
        let _reaped = self.0.wait();
    }
}

fn client_under_test() -> (String, String) {
    let spec = std::env::var("SILL_NESTED_CLIENT").unwrap_or_else(|_| "foot foot".to_owned());
    let (program, app_id) = spec
        .split_once(' ')
        .expect("SILL_NESTED_CLIENT is \"<program> <app_id>\"");
    (program.to_owned(), app_id.to_owned())
}

fn windows_of(toplevels: &watch::Receiver<CosmicWl>, app_id: &str) -> Vec<super::Toplevel> {
    toplevels
        .borrow()
        .toplevels
        .windows
        .iter()
        .filter(|w| w.app_id == app_id)
        .cloned()
        .collect()
}

#[test]
#[ignore = "needs a nested cosmic-comp"]
fn a_client_in_nested_cosmic_is_published_and_obeys_commands() {
    let nested = Nested::start();
    let (program, app_id) = client_under_test();
    let (workspaces, toplevels, control) = spawn(WaylandRoute::Socket(nested.socket.clone()));

    let groups = wait_for(WITHIN, || {
        let now = workspaces.borrow().workspaces.clone();
        now.groups
            .iter()
            .any(|g| !g.workspaces.is_empty())
            .then_some(now)
    })
    .expect("the workspace list has at least one entry");
    eprintln!("workspaces: {groups:?}");
    let before = windows_of(&toplevels, &app_id);

    let _client = Client(
        Command::new(&program)
            .env("WAYLAND_DISPLAY", &nested.display)
            .spawn()
            .unwrap_or_else(|e| panic!("{program} starts: {e}")),
    );
    let started = Instant::now();
    let window = wait_for(WITHIN, || {
        let now = windows_of(&toplevels, &app_id);
        (now.len() == before.len() + 1)
            .then(|| {
                now.into_iter()
                    .find(|w| !before.iter().any(|b| b.key == w.key))
            })
            .flatten()
    })
    .unwrap_or_else(|| {
        panic!(
            "one new {app_id} toplevel within {WITHIN:?}: {:?}",
            toplevels.borrow().toplevels
        )
    });
    eprintln!(
        "{app_id} appeared after {:?}: {window:?}",
        started.elapsed()
    );
    assert!(
        !window.workspaces.is_empty(),
        "the window names its workspace"
    );
    let known: Vec<_> = groups
        .groups
        .iter()
        .flat_map(|g| &g.workspaces)
        .map(|w| w.key)
        .collect();
    let now_known: Vec<_> = workspaces
        .borrow()
        .workspaces
        .groups
        .iter()
        .flat_map(|g| g.workspaces.iter().map(|w| w.key))
        .collect();
    assert!(
        window
            .workspaces
            .iter()
            .all(|k| known.contains(k) || now_known.contains(k)),
        "its workspace keys are published workspaces"
    );

    let state_of = |key| {
        toplevels
            .borrow()
            .toplevels
            .windows
            .iter()
            .find(|w| w.key == key)
            .map(|w| (w.window, w.activity))
    };
    control.send(WlCommand::Minimize(window.key)).expect("sent");
    wait_for(WITHIN, || {
        (state_of(window.key)?.0 == WindowState::Minimized).then_some(())
    })
    .expect("Minimize minimizes");
    control.send(WlCommand::Activate(window.key)).expect("sent");
    wait_for(WITHIN, || {
        (state_of(window.key)? == (WindowState::Normal, Activity::Active)).then_some(())
    })
    .expect("Activate restores and focuses");
    assert_eq!(
        toplevels.borrow().toplevels.windows[0].key,
        window.key,
        "and it is first in MRU order"
    );
    control.send(WlCommand::Close(window.key)).expect("sent");
    wait_for(WITHIN, || state_of(window.key).is_none().then_some(())).expect("Close closes");
}

/// Live: published coordinates and capabilities, pin and unpin when the workspace offers `Pin`, and
/// moving a window to another workspace when the group has two (cosmic-comp keeps an empty one
/// after the last occupied one).
#[test]
#[ignore = "needs a nested cosmic-comp"]
fn the_amended_workspace_commands_take_effect_in_nested_cosmic() {
    use super::{Pinning, WorkspaceCap, WorkspaceKey};

    let nested = Nested::start();
    let (program, app_id) = client_under_test();
    let (workspaces, toplevels, control) = spawn(WaylandRoute::Socket(nested.socket.clone()));
    let before = windows_of(&toplevels, &app_id);
    let _client = Client(
        Command::new(&program)
            .env("WAYLAND_DISPLAY", &nested.display)
            .spawn()
            .unwrap_or_else(|e| panic!("{program} starts: {e}")),
    );
    let window = wait_for(WITHIN * 2, || {
        windows_of(&toplevels, &app_id)
            .into_iter()
            .find(|w| !before.iter().any(|b| b.key == w.key) && !w.workspaces.is_empty())
    })
    .expect("a new toplevel on a workspace");
    let all = wait_for(WITHIN, || {
        let now = workspaces.borrow().workspaces.clone();
        now.groups
            .iter()
            .any(|g| g.workspaces.len() >= 2)
            .then_some(now)
    })
    .unwrap_or_else(|| workspaces.borrow().workspaces.clone());
    eprintln!("workspaces: {all:?}");
    let spaces: Vec<_> = all
        .groups
        .iter()
        .flat_map(|g| g.workspaces.clone())
        .collect();
    assert!(
        spaces.iter().all(|w| !w.coordinates.is_empty()),
        "every workspace publishes its coordinates"
    );
    assert!(
        spaces
            .iter()
            .all(|w| w.capabilities.contains(WorkspaceCap::Activate)),
        "cosmic-comp offers activate on every workspace"
    );

    let home = window.workspaces[0];
    let pinning_of = |key: WorkspaceKey| {
        workspaces
            .borrow()
            .workspaces
            .groups
            .iter()
            .flat_map(|g| g.workspaces.iter())
            .find(|w| w.key == key)
            .map(|w| w.pinning)
    };
    let offers_pin = spaces
        .iter()
        .find(|w| w.key == home)
        .is_some_and(|w| w.capabilities.contains(WorkspaceCap::Pin));
    if offers_pin {
        control.send(WlCommand::PinWorkspace(home)).expect("sent");
        wait_for(WITHIN, || {
            (pinning_of(home)? == Pinning::Pinned).then_some(())
        })
        .expect("PinWorkspace pins");
        control.send(WlCommand::UnpinWorkspace(home)).expect("sent");
        wait_for(WITHIN, || {
            (pinning_of(home)? == Pinning::Unpinned).then_some(())
        })
        .expect("UnpinWorkspace unpins");
        eprintln!("pin and unpin took effect on {home:?}");
    } else {
        eprintln!("{home:?} does not offer pin; pin not exercised");
    }

    let other = all
        .groups
        .iter()
        .find(|g| g.workspaces.iter().any(|w| w.key == home))
        .and_then(|g| g.workspaces.iter().find(|w| w.key != home))
        .map(|w| w.key);
    if let Some(other) = other {
        control
            .send(WlCommand::MoveToplevelToWorkspace {
                toplevel: window.key,
                workspace: other,
            })
            .expect("sent");
        wait_for(WITHIN, || {
            let now = windows_of(&toplevels, &app_id);
            now.iter()
                .find(|w| w.key == window.key)
                .is_some_and(|w| w.workspaces == vec![other])
                .then_some(())
        })
        .unwrap_or_else(|| {
            panic!(
                "the window moves to {other:?}: {:?}",
                toplevels.borrow().toplevels
            )
        });
        eprintln!("the window moved from {home:?} to {other:?}");
    } else {
        eprintln!("one workspace only; move-to-workspace not exercised");
    }
    control.send(WlCommand::Close(window.key)).expect("sent");
}
