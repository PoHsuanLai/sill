//! Live check against this machine's real tray: whatever `StatusNotifierItem`s are already on
//! the session bus. `#[ignore]`d because it needs the session bus: run with
//! `cargo test -p sill-services --test tray_live -- --ignored --nocapture`.

use std::time::Duration;

use sill_services::tray::{self, Tray};

#[test]
#[ignore = "needs a session bus with SNI items"]
fn at_least_one_status_notifier_item_appears() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, _control) = tray::spawn(runtime.handle());

    // The initial snapshot lands as soon as the client connects; menu layouts and icon
    // properties for each item can trickle in after, so give the bus a moment past the first
    // change before reading.
    let seeded = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), receiver.changed()).await
    });
    runtime.block_on(async { tokio::time::sleep(Duration::from_millis(500)).await });

    let tray: Tray = receiver.borrow().clone();
    for item in &tray.items {
        println!(
            "tray item: {:?} title={:?} status={:?} icon={:?} menu_entries={}",
            item.id,
            item.title,
            item.status,
            item.icon,
            item.menu.len()
        );
    }

    assert!(
        seeded.is_ok(),
        "no update arrived within 500ms of connecting to the session bus"
    );
    assert!(
        !tray.items.is_empty(),
        "expected at least one status notifier item on this session bus"
    );
}
