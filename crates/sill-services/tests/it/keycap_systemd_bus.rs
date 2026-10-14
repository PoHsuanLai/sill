//! The keycap service's systemd path against a stand-in `org.freedesktop.systemd1` on a private
//! bus playing the system bus, never the real one. The stand-in's `Manager` answers `GetUnit`
//! with a unit object whose `ActiveState` the test sets, or with `NoSuchUnit`, and records the
//! `ResetFailedUnit` and `RestartUnit` calls. The unit waits are shortened through
//! `Env::keycap_unit_wait`. The tests need a `dbus-daemon` and skip, saying so, without one.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sill_model::apps::AppIndex;
use sill_model::keycap::{Attention, KeycapCommand, KeycapState, notice_text};
use sill_model::notifications::{NotesState, NotificationsCommand};
use sill_model::windows::WindowModel;
use sill_services::Control;
use sill_services::env::{BusRoute, Buses, Env, UnitWait};
use sill_services::keycap::{KeycapLinks, KeycapService};
use sill_services::service::spawn;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::{mpsc, watch};
use zbus::interface;
use zbus::zvariant::OwnedObjectPath;

const SHORT: Duration = Duration::from_millis(400);
const LONG: Duration = Duration::from_secs(5);

const UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/keycap_2eservice";

/// What the stand-in systemd holds and heard.
#[derive(Debug, Default)]
struct Systemd {
    /// Whether the manager has the unit loaded.
    loaded: bool,
    /// The unit's `ActiveState`.
    active_state: String,
    /// The calls heard, in order, as `Method(args)`.
    calls: Vec<String>,
}

type Shared = Arc<Mutex<Systemd>>;

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, Systemd> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.systemd1")]
enum SystemdError {
    #[zbus(error)]
    ZBus(zbus::Error),
    NoSuchUnit(String),
}

struct Manager(Shared);

#[interface(name = "org.freedesktop.systemd1.Manager")]
impl Manager {
    fn get_unit(&self, name: String) -> Result<OwnedObjectPath, SystemdError> {
        let mut held = lock(&self.0);
        held.calls.push(format!("GetUnit({name})"));
        if held.loaded && name == "keycap.service" {
            Ok(OwnedObjectPath::try_from(UNIT_PATH).map_err(|e| SystemdError::ZBus(e.into()))?)
        } else {
            Err(SystemdError::NoSuchUnit(format!("Unit {name} not loaded.")))
        }
    }

    fn reset_failed_unit(&self, name: String) {
        lock(&self.0).calls.push(format!("ResetFailedUnit({name})"));
    }

    fn restart_unit(&self, name: String, mode: String) -> OwnedObjectPath {
        lock(&self.0)
            .calls
            .push(format!("RestartUnit({name}, {mode})"));
        OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1")
            .unwrap_or_else(|e| panic!("{e}"))
    }
}

struct Unit(Shared);

#[interface(name = "org.freedesktop.systemd1.Unit")]
impl Unit {
    #[zbus(property)]
    fn active_state(&self) -> String {
        lock(&self.0).active_state.clone()
    }
}

/// The stand-in on `bus`, serving as `org.freedesktop.systemd1`; dropped with the connection.
async fn serve(bus: &PrivateBus, loaded: bool, active_state: &str) -> (zbus::Connection, Shared) {
    let shared: Shared = Arc::new(Mutex::new(Systemd {
        loaded,
        active_state: active_state.to_owned(),
        calls: Vec::new(),
    }));
    let connection = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .name("org.freedesktop.systemd1")
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at("/org/freedesktop/systemd1", Manager(shared.clone()))
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(UNIT_PATH, Unit(shared.clone()))
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("the stand-in systemd: {e}"));
    (connection, shared)
}

/// The service with no daemon behind its socket, and the other services' ends.
struct Rig {
    state: watch::Receiver<KeycapState>,
    control: Control<KeycapCommand>,
    notices: mpsc::UnboundedReceiver<NotificationsCommand>,
    /// Held so the other services' channels stay open.
    _held: (
        watch::Sender<WindowModel>,
        watch::Sender<AppIndex>,
        watch::Sender<NotesState>,
        TempDir,
    ),
}

/// The service with `system` as its system bus and a socket path nobody listens on.
fn rig(system: BusRoute) -> Rig {
    let scratch = TempDir::new("keycap-systemd");
    let env = Env {
        keycap_socket: Some(scratch.path().join("keycap.sock")),
        buses: Buses::new(system, BusRoute::Absent),
        keycap_unit_wait: UnitWait {
            grace: Duration::from_millis(100),
            recheck: Duration::from_millis(100),
        },
        ..Env::isolated(scratch.path())
    };
    let (wl, wl_rx) = watch::channel(WindowModel::default());
    let (apps, apps_rx) = watch::channel(AppIndex::default());
    let (notes, notes_rx) = watch::channel(NotesState::default());
    let (notifications, notices) = Control::channel();
    let links = KeycapLinks {
        windows: wl_rx,
        apps: apps_rx,
        notifications,
        notes: notes_rx,
    };
    let handle = spawn::<KeycapService>(&tokio::runtime::Handle::current(), &env, links);
    Rig {
        state: handle.state,
        control: handle.control,
        notices,
        _held: (wl, apps, notes, scratch),
    }
}

/// A private bus and the stand-in on it, or `None` (said on stderr) to skip.
async fn with_systemd(
    loaded: bool,
    active_state: &str,
) -> Option<(PrivateBus, zbus::Connection, Shared)> {
    let bus = PrivateBus::start()?;
    let (connection, shared) = serve(&bus, loaded, active_state).await;
    Some((bus, connection, shared))
}

async fn until(state: &mut watch::Receiver<KeycapState>, want: KeycapState) {
    let reached = tokio::time::timeout(LONG, state.wait_for(|s| *s == want))
        .await
        .is_ok_and(|r| r.is_ok());
    assert!(
        reached,
        "gave up waiting for {want:?} at {:?}",
        state.borrow()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_unit_is_failed_and_the_person_is_told() {
    let Some((bus, _systemd, _shared)) = with_systemd(true, "failed").await else {
        return;
    };
    let mut rig = rig(BusRoute::Address(bus.address.clone()));
    until(&mut rig.state, KeycapState::Failed).await;
    let posted = tokio::time::timeout(LONG, rig.notices.recv())
        .await
        .unwrap_or_else(|_| panic!("no notice"))
        .unwrap_or_else(|| panic!("the notifications control closed"));
    let NotificationsCommand::Post(posting) = posted else {
        panic!("{posted:?}");
    };
    assert_eq!(posting.summary, notice_text(&Attention::Failed).summary);
    assert_eq!(posting.body.text(), notice_text(&Attention::Failed).body);
    assert_eq!(posting.replaces, None);
    assert!(posting.actions.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_unit_is_not_asked_about_within_the_grace_period() {
    let Some((bus, _systemd, shared)) = with_systemd(true, "failed").await else {
        return;
    };
    let scratch = TempDir::new("keycap-grace");
    let env = Env {
        keycap_socket: Some(scratch.path().join("keycap.sock")),
        buses: Buses::new(BusRoute::Address(bus.address.clone()), BusRoute::Absent),
        keycap_unit_wait: UnitWait {
            grace: Duration::from_secs(60),
            recheck: Duration::from_secs(60),
        },
        ..Env::isolated(scratch.path())
    };
    let (_wl, windows) = watch::channel(WindowModel::default());
    let (_apps, apps) = watch::channel(AppIndex::default());
    let (_notes, notes) = watch::channel(NotesState::default());
    let (notifications, mut notices) = Control::channel();
    let handle = spawn::<KeycapService>(
        &tokio::runtime::Handle::current(),
        &env,
        KeycapLinks {
            windows,
            apps,
            notifications,
            notes,
        },
    );
    tokio::time::sleep(SHORT).await;
    assert_eq!(*handle.state.borrow(), KeycapState::Unknown);
    assert!(lock(&shared).calls.is_empty());
    assert!(notices.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_unit_the_manager_has_not_loaded_is_not_installed_and_silent() {
    let Some((bus, _systemd, shared)) = with_systemd(false, "inactive").await else {
        return;
    };
    let mut rig = rig(BusRoute::Address(bus.address.clone()));
    until(&mut rig.state, KeycapState::NotInstalled).await;
    tokio::time::sleep(SHORT).await;
    assert_eq!(*rig.state.borrow(), KeycapState::NotInstalled);
    assert!(rig.notices.try_recv().is_err(), "nothing is posted");
    let calls = lock(&shared).calls.clone();
    assert_eq!(
        calls,
        ["GetUnit(keycap.service)"],
        "a machine with no keycap is asked once, then left alone"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_activating_unit_is_restarting_and_silent() {
    let Some((bus, _systemd, _shared)) = with_systemd(true, "activating").await else {
        return;
    };
    let mut rig = rig(BusRoute::Address(bus.address.clone()));
    until(&mut rig.state, KeycapState::Restarting).await;
    tokio::time::sleep(SHORT).await;
    assert_eq!(*rig.state.borrow(), KeycapState::Restarting);
    assert!(rig.notices.try_recv().is_err(), "nothing is posted");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_unit_is_read_again_and_a_later_failure_is_seen() {
    let Some((bus, _systemd, shared)) = with_systemd(true, "activating").await else {
        return;
    };
    let mut rig = rig(BusRoute::Address(bus.address.clone()));
    until(&mut rig.state, KeycapState::Restarting).await;
    assert!(rig.notices.try_recv().is_err());
    lock(&shared).active_state = "failed".to_owned();
    until(&mut rig.state, KeycapState::Failed).await;
    assert!(
        tokio::time::timeout(LONG, rig.notices.recv())
            .await
            .is_ok_and(|n| matches!(n, Some(NotificationsCommand::Post(_))))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn retry_resets_the_failure_then_restarts_the_unit() {
    let Some((bus, _systemd, shared)) = with_systemd(true, "inactive").await else {
        return;
    };
    let rig = rig(BusRoute::Address(bus.address.clone()));
    rig.control
        .send(KeycapCommand::Retry)
        .unwrap_or_else(|e| panic!("{e}"));
    let give_up = Instant::now() + LONG;
    let writes = || {
        lock(&shared)
            .calls
            .iter()
            .filter(|c| !c.starts_with("GetUnit"))
            .cloned()
            .collect::<Vec<_>>()
    };
    while writes().len() < 2 {
        assert!(Instant::now() < give_up, "heard only {:?}", writes());
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        writes(),
        [
            "ResetFailedUnit(keycap.service)",
            "RestartUnit(keycap.service, replace)"
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_system_bus_nothing_is_asked_and_the_state_stays_unknown() {
    // A bus with a stand-in exists, but the service is not told of it.
    let Some((_bus, _systemd, shared)) = with_systemd(true, "failed").await else {
        return;
    };
    let mut rig = rig(BusRoute::Absent);
    rig.control
        .send(KeycapCommand::Retry)
        .unwrap_or_else(|e| panic!("{e}"));
    tokio::time::sleep(SHORT * 2).await;
    assert_eq!(*rig.state.borrow(), KeycapState::Unknown);
    assert!(rig.notices.try_recv().is_err());
    assert!(lock(&shared).calls.is_empty());
}
