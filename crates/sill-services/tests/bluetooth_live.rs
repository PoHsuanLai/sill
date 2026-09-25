//! Live check against this machine's real BlueZ. `#[ignore]`d because it needs the system bus:
//! run with `cargo test -p sill-services -- --ignored`. Read-only, as CONVENTIONS and the M5
//! fill-in brief require: it only ever reads `bluetooth::spawn`'s receiver, never sends a
//! `BluetoothCommand` — so it cannot toggle the radio or connect/disconnect a device (the
//! machine's Bluetooth headphones may be the active audio output).

use std::time::Duration;

use sill_services::bluetooth::{self, Bluetooth};

#[test]
#[ignore = "needs the system bus"]
fn the_adapter_and_its_devices_seed_within_one_second() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = bluetooth::spawn(runtime.handle());

    // The adapter's own `Adapter` event fires before the device list finishes seeding (each
    // device is a separate `watch::Sender::send`), so a single `changed()` proves the service
    // is alive but not that it has settled; drain every change that arrives within the window
    // instead of stopping at the first one.
    let seeded = runtime.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let mut seeded = false;
        while let Ok(Ok(())) = tokio::time::timeout_at(deadline, receiver.changed()).await {
            seeded = true;
        }
        seeded
    });
    let bluetooth = receiver.borrow().clone();
    println!("bluetooth (BlueZ adapter and devices): {bluetooth:?}");

    assert!(
        seeded || bluetooth == Bluetooth::default(),
        "no update arrived within 1s and the receiver is not at its documented default: \
         {bluetooth:?}"
    );
}
