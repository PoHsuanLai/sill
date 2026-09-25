//! Live check against this machine's real NetworkManager: reads the wireless device and asks
//! for a scan. Read-only on purpose (FINDINGS "M5 freeze" safety note): never switches
//! networks, disconnects, forgets a network or toggles the radio. `#[ignore]`d because it needs
//! the system bus: run with `cargo test -p sill-services --test wifi_live -- --ignored
//! --nocapture`.

use std::time::Duration;

use sill_services::{Presence, Radio, Wifi, WifiCommand, network::wifi};

const SETTLE: Duration = Duration::from_secs(3);

#[test]
#[ignore = "needs the system bus"]
fn a_scan_returns_access_points() {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let (mut receiver, control) = wifi::spawn(runtime.handle());

    let seeded = runtime.block_on(async {
        tokio::time::timeout(Duration::from_millis(500), receiver.changed()).await
    });
    let wifi = receiver.borrow().clone();
    println!("wifi (seeded): {wifi:?}");
    assert!(
        seeded.is_ok() || wifi == Wifi::default(),
        "no update arrived within 500ms and the receiver is not at its documented default: \
         {wifi:?}"
    );

    if wifi.device == Presence::Absent {
        println!("wifi: no wireless device on this machine, nothing more to check");
        return;
    }
    if wifi.radio != Radio::On {
        println!("wifi: radio is {:?}, not requesting a scan", wifi.radio);
        return;
    }

    control.send(WifiCommand::Scan).expect("send Scan");
    let scanned = runtime.block_on(wait_for(&mut receiver, |w| !w.access_points.is_empty()));
    let wifi = receiver.borrow().clone();
    println!("wifi (after scan): {wifi:?}");
    assert!(
        scanned,
        "no access points appeared within {SETTLE:?} of a scan: {wifi:?}"
    );
}

/// Polls `receiver` until `Wifi` satisfies `want`, or [`SETTLE`] passes.
async fn wait_for(
    receiver: &mut tokio::sync::watch::Receiver<Wifi>,
    want: impl Fn(&Wifi) -> bool,
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
