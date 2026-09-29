//! Live checks of the auth service. Nothing here calls PAM with anyone's real password: the service
//! runs a `FakeAuthenticator` through `spawn_with`, and `PamHelper` runs against the stand-in
//! helper (`examples/auth_helper.rs`). The one PAM test runs only when `SILL_PAM_TEST_USER` and
//! `SILL_PAM_TEST_SECRET` name a throwaway account, and says it skipped otherwise. `#[ignore]`d:
//! run with
//!
//! ```sh
//! cargo test -p sill-services --test auth_live -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sill_services::auth::{
    self, Attempt, Attempts, Auth, AuthCommand, AuthFault, AuthPhase, Authenticator,
    FakeAuthenticator, FakeRule, FakeVerdict, Outcome, PamHelper, Secret, UserName, pam,
};
use sill_services::control::Control;
use sill_services::env::{Bus, BusRoute};
use tokio::sync::watch;

const RIGHT: &str = "right horse battery";

/// No system bus: the account's picture is never asked of the real AccountsService.
fn no_bus() -> Bus {
    Bus::system(BusRoute::Absent)
}

fn fake(delay_ms: u64) -> Arc<dyn Authenticator> {
    Arc::new(FakeAuthenticator {
        rule: FakeRule::Accepts(RIGHT.to_owned()),
        delay: Duration::from_millis(delay_ms),
    })
}

/// Waits until `auth` satisfies `want`, or panics after 5 s.
async fn until(auth: &mut watch::Receiver<Auth>, what: &str, want: impl Fn(&Auth) -> bool) -> Auth {
    let waited = tokio::time::timeout(Duration::from_secs(5), auth.wait_for(|a| want(a)))
        .await
        .map(|seen| seen.map(|seen| seen.clone()));
    match waited {
        Ok(Ok(seen)) => seen,
        _ => panic!("{what}: never happened; last {:?}", *auth.borrow()),
    }
}

fn send(control: &Control<AuthCommand>, command: AuthCommand) {
    control.send(command).unwrap_or_else(|e| panic!("{e}"));
}

/// One wrong secret, seen through `Checking` to `Wrong`.
async fn wrong_once(auth: &mut watch::Receiver<Auth>, control: &Control<AuthCommand>, n: u32) {
    send(
        control,
        AuthCommand::Submit(Secret::new(format!("wrong {n}"))),
    );
    until(auth, "checking", |a| a.phase == AuthPhase::Checking).await;
    until(auth, "a verdict", |a| a.failures == Attempts(n)).await;
}

/// No lockout: however many wrong secrets came before, the next one is checked at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live: spawns the auth service"]
async fn many_wrongs_never_lock_out_and_the_right_one_opens() {
    let (mut auth, control) =
        auth::spawn_with(&tokio::runtime::Handle::current(), no_bus(), fake(20));
    send(&control, AuthCommand::Reset);
    let loaded = until(&mut auth, "the account", |a| !a.account.user.0.is_empty()).await;
    println!(
        "account: {:?} ({:?})",
        loaded.account.user, loaded.account.name
    );
    for n in 1..=10 {
        wrong_once(&mut auth, &control, n).await;
        assert_eq!(auth.borrow().phase, AuthPhase::Wrong, "try {n}");
    }
    // A new lock keeps the count and waits for nothing.
    send(&control, AuthCommand::Reset);
    until(&mut auth, "idle", |a| a.phase == AuthPhase::Idle).await;
    assert_eq!(auth.borrow().failures, Attempts(10));
    let started = Instant::now();
    send(&control, AuthCommand::Submit(Secret::new(RIGHT)));
    until(&mut auth, "accepted", |a| a.phase == AuthPhase::Accepted).await;
    println!(
        "the right secret after ten wrong opened in {:?}",
        started.elapsed()
    );
    assert_eq!(auth.borrow().failures, Attempts(0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live: spawns the auth service"]
async fn busy_while_checking_then_right_then_reset() {
    let (mut auth, control) =
        auth::spawn_with(&tokio::runtime::Handle::current(), no_bus(), fake(300));
    send(&control, AuthCommand::Submit(Secret::new("nope")));
    send(&control, AuthCommand::Submit(Secret::new("nope again")));
    until(&mut auth, "wrong", |a| a.phase == AuthPhase::Wrong).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(
        auth.borrow().failures,
        Attempts(1),
        "the second submit was busy"
    );
    send(&control, AuthCommand::Submit(Secret::new(RIGHT)));
    until(&mut auth, "accepted", |a| a.phase == AuthPhase::Accepted).await;
    send(&control, AuthCommand::Reset);
    until(&mut auth, "idle", |a| a.phase == AuthPhase::Idle).await;
    // A check that a reset overtakes is dropped: its verdict never lands.
    send(&control, AuthCommand::Submit(Secret::new("stale")));
    until(&mut auth, "checking", |a| a.phase == AuthPhase::Checking).await;
    send(&control, AuthCommand::Reset);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(auth.borrow().phase, AuthPhase::Idle);
    assert_eq!(auth.borrow().failures, Attempts(0));
    // A made-up refusal goes through `Checking` like a real check.
    send(&control, AuthCommand::Fake(FakeVerdict::Refuse));
    until(&mut auth, "checking", |a| a.phase == AuthPhase::Checking).await;
    until(&mut auth, "wrong", |a| a.phase == AuthPhase::Wrong).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live: spawns the auth service"]
async fn an_unavailable_check_counts_nothing() {
    let failing = Arc::new(FakeAuthenticator {
        rule: FakeRule::Fails(AuthFault::ServiceError),
        delay: Duration::ZERO,
    });
    let (mut auth, control) =
        auth::spawn_with(&tokio::runtime::Handle::current(), no_bus(), failing);
    send(&control, AuthCommand::Submit(Secret::new("x")));
    until(&mut auth, "unavailable", |a| {
        a.phase == AuthPhase::Unavailable(AuthFault::ServiceError)
    })
    .await;
    assert_eq!(auth.borrow().failures, Attempts(0));
}

/// The stand-in helper, built next to this test binary.
fn stand_in() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let built = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "-p",
            "sill-services",
            "--example",
            "auth_helper",
        ])
        .status()
        .expect("cargo build the stand-in helper");
    assert!(built.success(), "building examples/auth_helper.rs");
    let exe = std::env::current_exe().expect("test binary path");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .join("examples")
        .join("auth_helper")
}

fn helper(program: PathBuf, args: &[&str], timeout: Duration) -> PamHelper {
    PamHelper {
        program,
        args: args.iter().map(|a| (*a).to_owned()).collect(),
        timeout,
    }
}

fn attempt(secret: &str) -> Attempt {
    Attempt {
        user: UserName("nobody-here".to_owned()),
        secret: Secret::new(secret),
    }
}

#[test]
#[ignore = "live: runs the stand-in helper"]
fn the_pam_helper_against_the_stand_in() {
    let program = stand_in();
    let long = Duration::from_secs(5);
    let accepting = helper(program.clone(), &["accept", RIGHT], long);
    let right = accepting.check(&attempt(RIGHT));
    assert_eq!(right.outcome, Outcome::Accepted, "{right:?}");
    assert_eq!(right.notes.len(), 1, "{right:?}");
    let wrong = accepting.check(&attempt("wrong"));
    assert_eq!(wrong.outcome, Outcome::Refused, "{wrong:?}");
    let garbage = helper(program.clone(), &["garbage"], long).check(&attempt(RIGHT));
    assert_eq!(
        garbage.outcome,
        Outcome::Unavailable(AuthFault::HelperFailed)
    );
    let started = Instant::now();
    let hung = helper(program, &["hang"], Duration::from_millis(500)).check(&attempt(RIGHT));
    let took = started.elapsed();
    println!("a hung helper was killed after {took:?}");
    assert_eq!(hung.outcome, Outcome::Unavailable(AuthFault::TimedOut));
    assert!(took < Duration::from_secs(2), "{took:?}");
    let missing = helper(PathBuf::from("/nonexistent/sill"), &[], long).check(&attempt(RIGHT));
    assert_eq!(missing.outcome, Outcome::Unavailable(AuthFault::NoHelper));
}

/// PAM for real, only for a throwaway account named in the environment; never the user's own.
#[test]
#[ignore = "live: PAM with a throwaway account"]
fn pam_with_a_throwaway_account() {
    let (Ok(user), Ok(secret)) = (
        std::env::var("SILL_PAM_TEST_USER"),
        std::env::var("SILL_PAM_TEST_SECRET"),
    ) else {
        println!("skipped: SILL_PAM_TEST_USER and SILL_PAM_TEST_SECRET name no throwaway account");
        return;
    };
    let service = pam::service(Path::exists);
    let check = |text: &str| {
        pam::check(
            &service,
            &Attempt {
                user: UserName(user.clone()),
                secret: Secret::new(text),
            },
        )
    };
    assert_eq!(check(&secret).outcome, Outcome::Accepted);
    assert_eq!(check("certainly not it").outcome, Outcome::Refused);
}
