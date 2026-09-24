//! Live check against the real session bus: the `badge_emitter` example stands in for an app,
//! and this test asserts the badge it publishes appears and, once the emitter is killed, clears
//! (`NameOwnerChanged` with an empty new owner). `#[ignore]`d because it needs a session bus:
//! run with `cargo test -p sill-services --test badges_live -- --ignored --nocapture`.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use sill_launcher::AppId;
use sill_services::badges::{self, Badges};

const DESKTOP_ID: &str = "org.sill.BadgeEmitterTest";
const SETTLE: Duration = Duration::from_millis(1_000);

#[test]
#[ignore = "needs the session bus"]
fn a_published_badge_appears_and_clears_when_the_emitter_exits() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = badges::spawn(runtime.handle());

    let mut emitter = spawn_emitter(&["--count", "7"]);

    let seen = runtime.block_on(wait_for(&mut receiver, |badges| {
        badges
            .apps
            .get(&app_id())
            .is_some_and(|badge| badge.shown_count() == Some(7))
    }));
    assert!(
        seen,
        "no badge for {DESKTOP_ID} with count 7 appeared within {SETTLE:?}"
    );

    kill_emitter(&mut emitter);

    let cleared = runtime.block_on(wait_for(&mut receiver, |badges| {
        !badges.apps.contains_key(&app_id())
    }));
    assert!(
        cleared,
        "the badge for {DESKTOP_ID} was not cleared within {SETTLE:?} of the emitter exiting"
    );
}

fn app_id() -> AppId {
    AppId(DESKTOP_ID.to_owned())
}

/// Runs `badge_emitter` for [`DESKTOP_ID`] via `cargo run`, with `extra_args` appended.
fn spawn_emitter(extra_args: &[&str]) -> Child {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut args = vec![
        "run",
        "--quiet",
        "-p",
        "sill-services",
        "--example",
        "badge_emitter",
        "--",
        DESKTOP_ID,
    ];
    args.extend_from_slice(extra_args);
    Command::new(cargo)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn badge_emitter")
}

/// Kills the emitter and waits for it to actually exit, so its bus connection has really
/// closed before the caller starts waiting on `NameOwnerChanged`.
fn kill_emitter(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Polls `receiver` until `Badges` satisfies `want`, or [`SETTLE`] passes.
async fn wait_for(
    receiver: &mut tokio::sync::watch::Receiver<Badges>,
    want: impl Fn(&Badges) -> bool,
) -> bool {
    if want(&receiver.borrow()) {
        return true;
    }
    let deadline = tokio::time::Instant::now() + SETTLE;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match tokio::time::timeout(remaining, receiver.changed()).await {
            Ok(Ok(())) => {
                if want(&receiver.borrow()) {
                    return true;
                }
            }
            _ => return false,
        }
    }
}
