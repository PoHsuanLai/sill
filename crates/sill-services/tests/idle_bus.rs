//! sill's own idle, live: the service on the nested cosmic-comp (`wayland-1`, or
//! `SILL_NESTED_WAYLAND`; pinged by connecting, never stopped; never `wayland-0`) with
//! `org.freedesktop.ScreenSaver` on a private bus and no logind at all (`BusRoute::Absent`), so
//! nothing can suspend or hint. Run it with
//! `cargo test -p sill-services --test idle_bus -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sill_model::battery::Battery;
use sill_model::idle::{
    DimLevel, DimPlan, Idle, IdleCommand, IdleLock, IdleMode, IdlePhase, IdlePolicy, IdleSecs,
    IdleTimes, Inhibition, LockGrace, LockedScreenOff, PowerPair, Timeout,
};
use sill_model::locker::{Lock, LockerCommand};
use sill_services::env::{BusRoute, Buses, Env, WaylandRoute};
use sill_services::idle::IdleLinks;
use sill_services::idle::IdleService;
use sill_services::service::spawn;
use sill_services::{AuthCommand, Control};
use sill_testkit::PrivateBus;
use tokio::sync::watch;

/// The nested compositor's socket, or `None` (the test then says so and passes).
fn nested() -> Option<PathBuf> {
    let path = match std::env::var_os("SILL_NESTED_WAYLAND") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("wayland-1"),
    };
    assert_ne!(
        path.file_name().and_then(|n| n.to_str()),
        Some("wayland-0"),
        "never the real session"
    );
    std::os::unix::net::UnixStream::connect(&path).ok()?;
    Some(path)
}

fn policy() -> IdlePolicy {
    let after = |s| Timeout::After(IdleSecs(s));
    IdlePolicy {
        mode: IdleMode::Own,
        times: IdleTimes {
            screen_off: PowerPair {
                ac: after(2),
                battery: after(2),
            },
            suspend: PowerPair {
                ac: Timeout::Never,
                battery: Timeout::Never,
            },
        },
        dim: DimPlan {
            lead: IdleSecs(1),
            level: DimLevel(50),
        },
        lock: IdleLock::AtScreenOff,
        locked_off: LockedScreenOff(IdleSecs(0)),
        grace: LockGrace(IdleSecs(5)),
    }
}

async fn until(idle: &mut watch::Receiver<Idle>, want: impl Fn(&Idle) -> bool) -> Idle {
    let give_up = Instant::now() + Duration::from_secs(8);
    loop {
        let now = *idle.borrow_and_update();
        if want(&now) {
            return now;
        }
        let left = give_up.saturating_duration_since(Instant::now());
        if tokio::time::timeout(left, idle.changed()).await.is_err() {
            panic!("gave up at {now:?}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "live: the nested cosmic-comp and a dbus-daemon"]
async fn idle_locks_at_screen_off_and_a_screensaver_inhibit_holds_it() {
    let Some(socket) = nested() else {
        eprintln!("no nested compositor: skipped");
        return;
    };
    let Some(bus) = PrivateBus::start() else {
        eprintln!("no private bus: skipped");
        return;
    };
    let (_lock_tx, lock) = watch::channel(Lock::default());
    let (_battery_tx, battery) = watch::channel(Battery::default());
    let (locker, mut locker_rx) = Control::<LockerCommand>::channel();
    let (auth, _auth_rx) = Control::<AuthCommand>::channel();
    let env = Env {
        wayland: WaylandRoute::Socket(socket),
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(Path::new("/nonexistent/sill-idle-live"))
    };
    let links = IdleLinks {
        lock,
        battery,
        locker,
        auth,
    };
    let idle = spawn::<IdleService>(&tokio::runtime::Handle::current(), &env, links);
    let (mut idle, control) = (idle.state, idle.control);
    assert_eq!(idle.borrow().mode, IdleMode::default(), "passive at first");
    control
        .send(IdleCommand::Use(policy()))
        .unwrap_or_else(|e| panic!("{e}"));
    until(&mut idle, |i| i.phase == IdlePhase::DisplaysOff).await;
    let asked = tokio::time::timeout(Duration::from_secs(2), locker_rx.recv()).await;
    assert!(
        matches!(asked, Ok(Some(LockerCommand::IdleLock))),
        "{asked:?}"
    );

    let client = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let reply = client
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "Inhibit",
            &("idle_bus", "a test"),
        )
        .await
        .unwrap_or_else(|e| panic!("Inhibit: {e}"));
    let cookie: u32 = reply.body().deserialize().unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(cookie, 0);
    let held = until(&mut idle, |i| i.inhibits == 1).await;
    assert_eq!(held.inhibition, Inhibition::Held);
    drop(client);
    let free = until(&mut idle, |i| i.inhibits == 0).await;
    assert_eq!(free.inhibition, Inhibition::Free);
}
