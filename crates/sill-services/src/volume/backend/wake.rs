//! A way to wake the thread that waits on the sound server. That thread sleeps inside libpulse's
//! poll until the server says something; a command from the rest of the shell reaches it by
//! making this descriptor readable, which the same poll watches.

use std::io;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

use rustix::event::{EventfdFlags, eventfd};

/// An eventfd: ringing it makes it readable until it is drained.
#[derive(Debug)]
pub(super) struct Wake(OwnedFd);

impl Wake {
    pub(super) fn new() -> io::Result<Wake> {
        let fd = eventfd(0, EventfdFlags::NONBLOCK | EventfdFlags::CLOEXEC)?;
        Ok(Wake(fd))
    }

    /// Make the descriptor readable. A ring that cannot be written (the counter is full) already
    /// leaves it readable.
    pub(super) fn ring(&self) {
        let _already_readable = rustix::io::write(&self.0, &1_u64.to_ne_bytes());
    }

    /// Make the descriptor quiet again.
    pub(super) fn drain(&self) {
        let mut count = [0_u8; 8];
        let _nothing_to_read = rustix::io::read(&self.0, &mut count[..]);
    }

    /// The descriptor libpulse's poll watches.
    pub(super) fn raw(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::Wake;

    fn count_in(wake: &Wake) -> Option<u64> {
        let mut bytes = [0_u8; 8];
        rustix::io::read(&wake.0, &mut bytes[..])
            .ok()
            .map(|_| u64::from_ne_bytes(bytes))
    }

    #[test]
    fn a_ring_is_there_to_read_until_drained() {
        let wake = Wake::new().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(count_in(&wake), None, "quiet at first");
        wake.ring();
        wake.ring();
        wake.drain();
        assert_eq!(count_in(&wake), None, "two rings are gone in one drain");
        wake.ring();
        assert_eq!(count_in(&wake), Some(1), "a ring after a drain is there");
    }
}
