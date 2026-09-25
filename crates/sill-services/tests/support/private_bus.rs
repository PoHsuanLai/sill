//! A private `dbus-daemon` for the notifications tests: its own socket, no service activation
//! (`dev/private-bus.conf`), stopped when dropped. No notifications test may reach the real
//! session bus, whose `org.freedesktop.Notifications` belongs to the person's own daemon.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// A running private bus.
#[derive(Debug)]
pub struct PrivateBus {
    child: Child,
    /// The address a client connects to (`unix:path=/tmp/dbus-...,guid=...`).
    pub address: String,
}

/// The config every private bus runs with.
pub fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dev/private-bus.conf")
}

impl PrivateBus {
    /// Start one; panics when `dbus-daemon` is missing (these tests are `#[ignore]`d live tests).
    pub fn start() -> PrivateBus {
        let mut child = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config().display()))
            .args(["--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("starting dbus-daemon: {e}"));
        let stdout = child
            .stdout
            .take()
            .unwrap_or_else(|| panic!("dbus-daemon has no stdout"));
        let mut address = String::new();
        BufReader::new(stdout)
            .read_line(&mut address)
            .unwrap_or_else(|e| panic!("reading the bus address: {e}"));
        let address = address.trim().to_owned();
        assert!(address.starts_with("unix:"), "bus address {address:?}");
        PrivateBus { child, address }
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
