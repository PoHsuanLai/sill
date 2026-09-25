//! Live check against the session bus, using the lane's own fake MPRIS player
//! (`examples/mpris_player.rs`) rather than a real one, under its own bus name (FINDINGS "M5
//! freeze": never control a real player). `#[ignore]`d because it needs the session bus: run
//! with `cargo test -p sill-services --test media_live -- --ignored --nocapture`.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use sill_services::{Media, MediaCommand, MediaVerb, Playback, PlayerId, now_playing};

const SUFFIX: &str = "sill_test_media_live";
const SETTLE: Duration = Duration::from_secs(2);

#[test]
#[ignore = "needs the session bus"]
fn the_fake_player_becomes_current_and_obeys_play_pause() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, control) = now_playing::spawn(runtime.handle());
    let id = PlayerId(format!("org.mpris.MediaPlayer2.{SUFFIX}"));

    let mut player = spawn_player();

    let seen = runtime.block_on(wait_for(&mut receiver, |media| {
        media.current().is_some_and(|p| p.id == id)
    }));
    let media = receiver.borrow().clone();
    println!("media (fake player found): {media:?}");
    assert!(
        seen,
        "the fake player never became current within {SETTLE:?}: {media:?}"
    );

    control
        .send(MediaCommand::Press {
            player: id.clone(),
            verb: MediaVerb::PlayPause,
        })
        .expect("send Press");

    let paused = runtime.block_on(wait_for(&mut receiver, |media| {
        media
            .players
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.playback)
            == Some(Playback::Paused)
    }));
    let media = receiver.borrow().clone();
    println!("media (after PlayPause): {media:?}");
    assert!(
        paused,
        "PlayPause never reached the fake player within {SETTLE:?}: {media:?}"
    );

    kill_player(&mut player);
}

/// Runs `mpris_player` under [`SUFFIX`] via `cargo run`.
fn spawn_player() -> Child {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    Command::new(cargo)
        .args([
            "run",
            "--quiet",
            "-p",
            "sill-services",
            "--example",
            "mpris_player",
            "--",
            SUFFIX,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mpris_player")
}

/// Kills the player and waits for it to actually exit, so its bus connection has really closed.
fn kill_player(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Polls `receiver` until `Media` satisfies `want`, or [`SETTLE`] passes.
async fn wait_for(
    receiver: &mut tokio::sync::watch::Receiver<Media>,
    want: impl Fn(&Media) -> bool,
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
