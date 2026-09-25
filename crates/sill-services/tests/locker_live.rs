//! Live checks of the locker service against the NESTED compositor (`wayland-1`), never the
//! real session: every locker here gets `WAYLAND_DISPLAY=wayland-1` in its plan, the test
//! refuses to run when that socket is missing, and the service runs with
//! `Triggers::IgnoreLogind`, so logind's signals for the real session are never acted on and
//! nothing here calls logind at all. `#[ignore]`d: run with
//!
//! ```sh
//! cargo test -p sill-services --test locker_live -- --ignored --test-threads=1 --nocapture
//! ```
//!
//! 1. The stand-in locker (`examples/session_locker.rs`, password-free) takes
//!    `ext-session-lock-v1`, reports on stdout, holds, unlocks and exits: sill sees Locking,
//!    Locked and Unlocked, and a second Lock while locked starts nothing.
//! 2. cosmic-greeter, as sill runs it (one-shot, no system bus): sill sees it lock through its
//!    journal line, the compositor refuses a second lock while it runs, and killing it is seen
//!    as an exit. It cannot be unlocked without the user's password, so the test kills it and
//!    then clears the lock it leaves behind with the stand-in (cosmic-comp keeps a session
//!    locked after its locker dies until another client locks and unlocks).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use sill_services::locker::{
    self, KnownLocker, Lock, LockPhase, LockerCommand, LockerMode, LockerPlan, Readiness, Triggers,
};
use sill_services::programs;
use tokio::sync::watch;

const NESTED: &str = "wayland-1";

fn nested_socket() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR");
    Path::new(&runtime).join(NESTED)
}

/// The stand-in locker, built next to this test binary.
fn stand_in() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let built = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "-p",
            "sill-services",
            "--example",
            "session_locker",
        ])
        .status()
        .expect("cargo build the stand-in locker");
    assert!(built.success(), "building examples/session_locker.rs");
    let exe = std::env::current_exe().expect("test binary path");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .join("examples")
        .join("session_locker")
}

fn nested_env() -> Vec<(String, String)> {
    vec![("WAYLAND_DISPLAY".to_owned(), NESTED.to_owned())]
}

/// Refuses to run without the nested compositor, and waits (up to five minutes) while another
/// sill daemon is using it, so a lock never lands in the middle of someone else's run.
fn preflight() {
    assert!(
        nested_socket().exists(),
        "no nested compositor at {}: start it with dev/nested.sh",
        nested_socket().display()
    );
    let until = Instant::now() + Duration::from_secs(300);
    while !programs::running("sill").is_empty() {
        assert!(Instant::now() < until, "another sill daemon kept running");
        println!("waiting: another sill daemon is running");
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Runs the stand-in once with `hold_ms`; its exit code (0 locked and unlocked, 3 refused).
fn stand_in_once(hold_ms: u64) -> Option<i32> {
    Command::new(stand_in())
        .arg(hold_ms.to_string())
        .env("WAYLAND_DISPLAY", NESTED)
        .status()
        .ok()
        .and_then(|status| status.code())
}

/// Waits for `rx` to reach `phase`, logging every value seen; the value, or a panic after
/// `within`.
async fn reach(
    rx: &mut watch::Receiver<Lock>,
    phase: LockPhase,
    within: Duration,
    seen: &mut Vec<Lock>,
) -> Lock {
    let deadline = tokio::time::Instant::now() + within;
    loop {
        let now = *rx.borrow_and_update();
        if seen.last() != Some(&now) {
            println!("  sill sees {now:?}");
            seen.push(now);
        }
        if now.phase == phase {
            return now;
        }
        let changed = tokio::time::timeout_at(deadline, rx.changed()).await;
        assert!(
            matches!(changed, Ok(Ok(()))),
            "no {phase:?} within {within:?}; saw {seen:?}"
        );
    }
}

#[test]
#[ignore = "needs the nested compositor on wayland-1"]
fn the_stand_in_locks_and_unlocks_the_nested_compositor() {
    preflight();
    let program = stand_in();
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let (mut rx, control) = locker::spawn(runtime.handle());
        control
            .send(LockerCommand::Use(LockerMode::Run {
                plan: LockerPlan {
                    program,
                    args: vec!["1500".to_owned()],
                    env: nested_env(),
                    readiness: Readiness::StdoutLine,
                    process_name: Some("session_locker".to_owned()),
                },
                triggers: Triggers::IgnoreLogind,
            }))
            .expect("configure");
        let mut seen = Vec::new();
        let started = Instant::now();
        control.send(LockerCommand::Lock).expect("lock");
        let locked = reach(
            &mut rx,
            LockPhase::Locked,
            Duration::from_secs(5),
            &mut seen,
        )
        .await;
        println!("locked after {:?}", started.elapsed());
        control.send(LockerCommand::Lock).expect("lock again");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            *rx.borrow(),
            locked,
            "a second Lock while locked must not start another locker"
        );
        reach(
            &mut rx,
            LockPhase::Unlocked,
            Duration::from_secs(5),
            &mut seen,
        )
        .await;
        println!("unlocked after {:?}", started.elapsed());
        let phases: Vec<LockPhase> = seen.iter().map(|lock| lock.phase).collect();
        assert!(
            phases.ends_with(&[LockPhase::Locked, LockPhase::Unlocked]),
            "{phases:?}"
        );
    });
    assert_eq!(stand_in_once(0), Some(0), "the compositor is left unlocked");
}

/// Kills the greeter (if still running) and clears the lock it leaves, even when an assertion
/// failed on the way.
struct Cleanup(Option<u32>);

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = Command::new("kill").arg(pid.to_string()).status();
            std::thread::sleep(Duration::from_millis(300));
        }
        let code = stand_in_once(0);
        println!("recovery lock-and-unlock: exit {code:?}");
    }
}

#[test]
#[ignore = "needs the nested compositor on wayland-1 and cosmic-greeter"]
fn cosmic_greeter_locks_the_nested_compositor_and_its_exit_is_seen() {
    preflight();
    let Some(path) = programs::on_path(KnownLocker::CosmicGreeter.program()) else {
        println!("cosmic-greeter is not installed; nothing to check");
        return;
    };
    let _ = stand_in();
    let mut plan = KnownLocker::CosmicGreeter.plan(path);
    plan.env.extend(nested_env());
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let mut cleanup = Cleanup(None);
    runtime.block_on(async {
        let (mut rx, control) = locker::spawn(runtime.handle());
        control
            .send(LockerCommand::Use(LockerMode::Run {
                plan,
                triggers: Triggers::IgnoreLogind,
            }))
            .expect("configure");
        let mut seen = Vec::new();
        let started = Instant::now();
        control.send(LockerCommand::Lock).expect("lock");
        let locked = reach(
            &mut rx,
            LockPhase::Locked,
            Duration::from_secs(15),
            &mut seen,
        )
        .await;
        let took = started.elapsed();
        println!("cosmic-greeter reported locked after {took:?}");
        cleanup.0 = locked.pid.map(|pid| pid.0);
        assert!(
            took < Duration::from_millis(locker::READY_GIVE_UP_MS),
            "locked by the journal line, not by giving up"
        );

        let refused = tokio::task::spawn_blocking(|| stand_in_once(0))
            .await
            .expect("stand-in");
        assert_eq!(
            refused,
            Some(3),
            "the compositor refuses a second lock while cosmic-greeter holds it"
        );

        let pid = locked.pid.expect("a pid").0;
        let killed = Command::new("kill").arg(pid.to_string()).status();
        assert!(killed.is_ok_and(|s| s.success()), "kill {pid}");
        reach(
            &mut rx,
            LockPhase::Unlocked,
            Duration::from_secs(5),
            &mut seen,
        )
        .await;
        cleanup.0 = None;
        println!("sill saw cosmic-greeter exit");
    });
    drop(cleanup);
    assert_eq!(
        stand_in_once(0),
        Some(0),
        "the compositor is unlocked after the recovery"
    );
}
