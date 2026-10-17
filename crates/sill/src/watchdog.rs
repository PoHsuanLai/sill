//! Telling the service manager the shell is alive. The unit is `Type=notify` with a watchdog: the
//! manager restarts the shell when it hears nothing for `WatchdogSec`. `READY=1` is sent once the
//! host has started, and `WATCHDOG=1` every [`PING_EVERY`] by a thread of its own, but only while
//! the host loop's pulse (`shell_host::Host::pulse`) has moved since the last look: a loop stuck
//! in a blocking call stops the pings, and the manager restarts the shell; an idle loop still
//! beats, so a quiet desktop is never restarted. Without `NOTIFY_SOCKET` (a run by hand) every
//! send does nothing.

use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::path::PathBuf;
use std::time::Duration;

/// How often the watcher looks at the pulse: half of the unit's `WatchdogSec` (30 s), so one late
/// look still lands before the manager gives up.
pub const PING_EVERY: Duration = Duration::from_secs(15);

/// How often the host loop beats: several beats fit in one [`PING_EVERY`].
pub const BEAT_EVERY_MS: u32 = 5000;

/// What the manager is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Message {
    Ready,
    Alive,
}

impl Message {
    fn text(self) -> &'static [u8] {
        match self {
            Message::Ready => b"READY=1",
            Message::Alive => b"WATCHDOG=1",
        }
    }
}

/// Where the manager listens.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Address {
    Path(PathBuf),
    /// An abstract socket: `NOTIFY_SOCKET` began with `@`.
    Abstract(Vec<u8>),
}

/// The address `NOTIFY_SOCKET` names: a path, or an abstract name after a leading `@`. None for
/// an empty value.
fn address_of(value: &str) -> Option<Address> {
    match value.strip_prefix('@') {
        Some("") => None,
        Some(name) => Some(Address::Abstract(name.as_bytes().to_vec())),
        None if value.is_empty() => None,
        None => Some(Address::Path(PathBuf::from(value))),
    }
}

/// Where the shell's word to the manager goes; nowhere when there is no manager listening.
#[derive(Debug, Clone, Default)]
pub struct Notifier {
    address: Option<Address>,
}

impl Notifier {
    /// The notifier `value` of `NOTIFY_SOCKET` makes.
    pub fn from_value(value: Option<&str>) -> Notifier {
        Notifier {
            address: value.and_then(address_of),
        }
    }

    /// The notifier of this process's environment.
    pub fn from_process() -> Notifier {
        Notifier::from_value(std::env::var("NOTIFY_SOCKET").ok().as_deref())
    }

    /// `READY=1`: the shell has started.
    pub fn ready(&self) {
        self.send(Message::Ready);
    }

    fn send(&self, message: Message) {
        let Some(address) = &self.address else {
            return;
        };
        if let Err(error) = deliver(address, message.text()) {
            eprintln!("sill: telling the service manager {message:?}: {error}");
        }
    }
}

fn deliver(address: &Address, text: &[u8]) -> std::io::Result<()> {
    let socket = UnixDatagram::unbound()?;
    match address {
        Address::Path(path) => socket.send_to(text, path).map(|_| ()),
        Address::Abstract(name) => {
            let addr = SocketAddr::from_abstract_name(name)?;
            socket.send_to_addr(text, &addr).map(|_| ())
        }
    }
}

/// Whether the watcher pings: the loop has beaten since the last look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ping {
    Send,
    Skip,
}

/// A ping when `now` beats differ from the `last` look's: the count only moves while the loop
/// dispatches.
fn ping_for(last: u64, now: u64) -> Ping {
    match now != last {
        true => Ping::Send,
        false => Ping::Skip,
    }
}

/// Start the watcher: every [`PING_EVERY`] it reads `beats` and, if the count moved, pings. It
/// runs for the life of the process. Nothing starts without a manager to tell.
pub fn spawn(notifier: Notifier, beats: impl Fn() -> u64 + Send + 'static) {
    if notifier.address.is_none() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("watchdog".to_owned())
        .spawn(move || {
            let mut last = beats();
            loop {
                std::thread::sleep(PING_EVERY);
                let now = beats();
                if ping_for(last, now) == Ping::Send {
                    notifier.send(Message::Alive);
                }
                last = now;
            }
        });
    if let Err(error) = spawned {
        eprintln!("sill: no watchdog thread: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixDatagram;
    use std::path::PathBuf;

    use sill_testkit::TempDir;

    use super::{Address, Notifier, Ping, address_of, ping_for};

    #[test]
    fn a_ping_goes_only_when_the_beats_moved() {
        const CASES: &[(u64, u64, Ping)] = &[
            (0, 1, Ping::Send),
            (7, 9, Ping::Send),
            (7, 7, Ping::Skip),
            (0, 0, Ping::Skip),
        ];
        for &(last, now, want) in CASES {
            assert_eq!(ping_for(last, now), want, "{last} -> {now}");
        }
    }

    #[test]
    fn the_notify_socket_names_a_path_or_an_abstract_name() {
        const CASES: &[(&str, Option<&str>)] = &[
            ("/run/user/1000/systemd/notify", Some("path")),
            ("@abstract-name", Some("abstract")),
            ("", None),
            ("@", None),
        ];
        for &(value, want) in CASES {
            let got = address_of(value).map(|a| match a {
                Address::Path(_) => "path",
                Address::Abstract(_) => "abstract",
            });
            assert_eq!(got, want, "{value:?}");
        }
        assert_eq!(address_of("@abc"), Some(Address::Abstract(b"abc".to_vec())));
    }

    #[test]
    fn ready_reaches_a_listening_socket_and_no_socket_is_a_no_op() {
        let dir = TempDir::new("watchdog");
        let path: PathBuf = dir.path().join("notify");
        let listener = UnixDatagram::bind(&path).unwrap_or_else(|e| panic!("{e}"));
        let notifier = Notifier::from_value(path.to_str());
        notifier.ready();
        let mut buf = [0_u8; 32];
        let n = listener.recv(&mut buf).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(&buf[..n], b"READY=1");
        Notifier::from_value(None).ready();
    }
}
