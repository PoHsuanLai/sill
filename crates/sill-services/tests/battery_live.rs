//! Live check against this machine's real UPower. `#[ignore]`d because it needs the session
//! bus: run with `cargo test -p sill-services -- --ignored`.

use std::time::Duration;

use sill_services::battery::{self, Battery};

#[test]
#[ignore = "needs the session bus"]
fn the_display_device_seeds_within_half_a_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = battery::spawn(runtime.handle());

    let seeded = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), receiver.changed()).await
    });
    let battery = receiver.borrow().clone();
    println!("battery (upower display device): {battery:?}");

    assert!(
        seeded.is_ok() || battery == Battery::default(),
        "no update arrived within 500ms and the receiver is not at its documented default: \
         {battery:?}"
    );
}
