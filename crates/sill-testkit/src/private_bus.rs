//! A private `dbus-daemon` for the bus tests: its own socket, no service activation
//! (`dev/private-bus.conf`), stopped when dropped. No test may reach the real session bus, whose
//! names belong to the person's own daemons.

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
fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dev/private-bus.conf")
}

impl PrivateBus {
    /// Start one. Without a `dbus-daemon` there is no bus to test against and the answer is
    /// `None`, announced on stderr so the skip is visible; a test never falls back to the real
    /// session bus.
    pub fn start() -> Option<PrivateBus> {
        let spawned = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config().display()))
            .args(["--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                eprintln!("SKIPPED: no private bus, dbus-daemon did not start ({e})");
                return None;
            }
        };
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
        Some(PrivateBus { child, address })
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
