//! Live checks against this machine's real power-profiles-daemon. `#[ignore]`d because they
//! need the system bus: run with `cargo test -p sill-services -- --ignored`.

use std::time::Duration;

use sill_services::power_profile::{self, PowerProfiles, ProfileCommand};

/// Read-only: never sends a command, only reads the seeded receiver.
#[test]
#[ignore = "needs the system bus"]
fn the_active_profile_seeds_within_one_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = power_profile::spawn(runtime.handle());

    // `seed` publishes `Found`, `Active`, `Offered` and `Degraded` as four separate sends;
    // drain every change in the window rather than stopping at the first one, or `offered`
    // would still be empty when this reads it.
    let seeded = runtime.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let mut seeded = false;
        while let Ok(Ok(())) = tokio::time::timeout_at(deadline, receiver.changed()).await {
            seeded = true;
        }
        seeded
    });
    let profiles = receiver.borrow().clone();
    println!("power profiles (power-profiles-daemon): {profiles:?}");

    assert!(
        seeded || profiles == PowerProfiles::default(),
        "no update arrived within 1s and the receiver is not at its documented default: \
         {profiles:?}"
    );
}

/// Sets a profile and restores the original one immediately after (task brief: "a live test may
/// set a profile only if it restores the original immediately after"), so this machine's power
/// profile is unchanged once the test ends. `#[ignore]`d by default in addition to needing the
/// bus.
#[test]
#[ignore = "needs the system bus; changes and restores this machine's power profile"]
fn setting_a_profile_round_trips() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, control) = power_profile::spawn(runtime.handle());

    runtime.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        while let Ok(Ok(())) = tokio::time::timeout_at(deadline, receiver.changed()).await {}
    });
    let before = receiver.borrow().clone();
    println!("power profiles before: {before:?}");
    let Some(&other) = before.offered.iter().find(|p| **p != before.active) else {
        println!("only one profile offered; nothing to round-trip");
        return;
    };

    control
        .send(ProfileCommand::Set(other))
        .expect("service running");
    runtime.block_on(async {
        let _ = tokio::time::timeout(Duration::from_secs(1), receiver.changed()).await;
    });
    println!(
        "power profiles after Set({other:?}): {:?}",
        receiver.borrow().clone()
    );

    control
        .send(ProfileCommand::Set(before.active))
        .expect("service running");
    runtime.block_on(async {
        let _ = tokio::time::timeout(Duration::from_secs(1), receiver.changed()).await;
    });
    let restored = receiver.borrow().clone();
    println!("power profiles restored: {restored:?}");
    assert_eq!(
        restored.active, before.active,
        "must end on the profile it started on"
    );
}
