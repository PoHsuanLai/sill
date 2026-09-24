//! Live check against this machine's real pipewire-pulse. `#[ignore]`d because it needs the
//! PulseAudio-compatible server: run with `cargo test -p sill-services -- --ignored`.

use std::time::Duration;

use sill_services::volume::{self, Volume};

#[test]
#[ignore = "needs the session bus"]
fn the_default_sink_seeds_within_half_a_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = volume::spawn(runtime.handle());

    let seeded = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), receiver.changed()).await
    });
    let volume = receiver.borrow().clone();
    println!("volume (default sink over pipewire-pulse): {volume:?}");

    assert!(
        seeded.is_ok() || volume == Volume::default(),
        "no update arrived within 500ms and the receiver is not at its documented default: \
         {volume:?}"
    );
}
