//! sill's Bluetooth service against `dev/fake-bluez.py` on a private bus, never the system bus:
//! sill starts first, BlueZ appears late, is killed, and comes back. The test needs a
//! `dbus-daemon` and python's `dbus` and `gi`, and skips, saying so, without them.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use sill_model::bluetooth::{Bluetooth, BtPower};
use sill_model::units::Presence;
use sill_services::bluetooth::BluetoothService;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::spawn;
use sill_testkit::PrivateBus;
use tokio::sync::watch;

fn fake_bluez() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dev/fake-bluez.py")
}

/// The fake started on `bus`, once it has printed that it owns the name; `None` (skip) when
/// python cannot run it.
fn start_fake(bus: &PrivateBus, log: &Path) -> Option<Child> {
    let mut child = Command::new("python3")
        .arg(fake_bluez())
        .arg(&bus.address)
        .arg(log)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(stdout), &mut line).ok()?;
    if line.starts_with("fake-bluez: org.bluez") {
        Some(child)
    } else {
        let _ = child.kill();
        let _ = child.wait();
        None
    }
}

fn stop(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

async fn until(
    rx: &mut watch::Receiver<Bluetooth>,
    what: &str,
    ok: impl FnMut(&Bluetooth) -> bool,
) {
    let waited = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(ok))
        .await
        .map(|r| r.map(|_| ()));
    match waited {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{what}: {e}"),
        Err(_) => panic!("timed out waiting for {what}: {:?}", rx.borrow()),
    }
}

fn on(bluetooth: &Bluetooth) -> bool {
    bluetooth.presence == Presence::Present
        && bluetooth.power == BtPower::On
        && bluetooth.devices.len() == 1
}

#[test]
fn bluez_starting_late_dying_and_returning_is_followed() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("sill-bt-bus-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap_or_else(|e| panic!("{e}"));
    let log = scratch.join("powered.log");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = Env {
            buses: Buses::new(
                BusRoute::Address(bus.address.clone()),
                BusRoute::Address(bus.address.clone()),
            ),
            ..Env::isolated(&scratch)
        };
        // sill first: no org.bluez yet.
        let service = spawn::<BluetoothService>(&tokio::runtime::Handle::current(), &env, ());
        let mut rx = service.state;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(rx.borrow().presence, Presence::Absent);

        // BlueZ starts late.
        let Some(fake) = start_fake(&bus, &log) else {
            eprintln!("SKIPPED: fake-bluez.py cannot run (python dbus/gi missing)");
            return;
        };
        until(&mut rx, "On after a late BlueZ", on).await;

        // BlueZ dies: stale On must not stay.
        stop(fake);
        until(&mut rx, "Absent after BlueZ died", |b| {
            b.presence == Presence::Absent && b.devices.is_empty()
        })
        .await;

        // BlueZ restarts.
        let fake = start_fake(&bus, &log).unwrap_or_else(|| panic!("fake restart"));
        until(&mut rx, "On after BlueZ restarted", on).await;
        stop(fake);
        drop(service.control);
    });
    let _ = std::fs::remove_dir_all(&scratch);
}
