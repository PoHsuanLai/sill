//! The two ends of a selection's pipe, on the live loop without blocking it: reading an
//! offered selection whole (up to a limit, within a deadline) and writing an entry the service
//! offers to whoever pastes it. Both are calloop sources woken by readiness, so a source that
//! is the service itself (an entry offered back, which the compositor reports as a new
//! selection) is written and read by the same loop, and a stalled peer is dropped at its
//! deadline rather than holding the loop. The bytes pass through and are never printed.

use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::time::Duration;

use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{
    Interest, LoopHandle, Mode, PostAction, RegistrationToken,
};

use super::content::ReadLimit;

/// How long a source has to write its selection before the read is given up.
pub(crate) const READ_DEADLINE: Duration = Duration::from_secs(2);

/// How long a paste has to take an offered entry before the write is given up.
const WRITE_DEADLINE: Duration = Duration::from_secs(5);

/// One read at a time: which one (so a late callback of an abandoned read is ignored).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadId(pub(crate) u64);

/// How a read ended.
#[derive(Debug)]
pub(crate) enum ReadEnd {
    /// The source closed its end: everything it wrote.
    Whole(Vec<u8>),
    /// More than the limit was written.
    OverLimit,
    /// The pipe failed.
    Broken(ErrorKind),
}

/// A read in progress: its pipe's source and its deadline's.
#[derive(Debug)]
pub(crate) struct Reading {
    pub(crate) id: ReadId,
    pub(crate) pipe: RegistrationToken,
    pub(crate) deadline: RegistrationToken,
}

/// What the loop's state does when a read ends or times out.
pub(crate) trait ReadOwner: 'static {
    fn read_ended(&mut self, id: ReadId, end: ReadEnd);
    fn read_timed_out(&mut self, id: ReadId);
}

/// Read `pipe` until the writer closes it, at most `limit` bytes, within [`READ_DEADLINE`].
pub(crate) fn read_whole<S: ReadOwner>(
    handle: &LoopHandle<'static, S>,
    pipe: File,
    id: ReadId,
    limit: ReadLimit,
) -> Result<Reading, ErrorKind> {
    let mut bytes = Vec::new();
    let source = Generic::new(pipe, Interest::READ, Mode::Level);
    let pipe = handle
        .insert_source(source, move |_, file, state: &mut S| {
            match step_read(&mut &**file, &mut bytes, limit) {
                Some(end) => {
                    state.read_ended(id, end);
                    Ok(PostAction::Remove)
                }
                None => Ok(PostAction::Continue),
            }
        })
        .map_err(|error| kind_of(error.error))?;
    let deadline = handle
        .insert_source(Timer::from_duration(READ_DEADLINE), move |_, _, state| {
            state.read_timed_out(id);
            TimeoutAction::Drop
        })
        .map_err(|error| kind_of(error.error))?;
    Ok(Reading { id, pipe, deadline })
}

/// One read's worth (the pipe is readable, so this does not block): `Some` once it has ended.
fn step_read(pipe: &mut impl Read, bytes: &mut Vec<u8>, limit: ReadLimit) -> Option<ReadEnd> {
    let mut chunk = [0_u8; 1 << 16];
    match pipe.read(&mut chunk) {
        Ok(0) => Some(ReadEnd::Whole(std::mem::take(bytes))),
        Ok(n) => {
            bytes.extend_from_slice(&chunk[..n]);
            let over = u64::try_from(bytes.len()).map_or(true, |len| len > limit.0);
            over.then(|| {
                bytes.clear();
                ReadEnd::OverLimit
            })
        }
        Err(error) if error.kind() == ErrorKind::Interrupted => None,
        Err(error) => Some(ReadEnd::Broken(error.kind())),
    }
}

/// Write `bytes` into `fd` as the reader takes them, then close it; given up after
/// [`WRITE_DEADLINE`] or when the reader goes.
pub(crate) fn write_whole<S: 'static>(
    handle: &LoopHandle<'static, S>,
    fd: OwnedFd,
    bytes: Arc<[u8]>,
) -> Result<(), ErrorKind> {
    let mut written = 0_usize;
    let source = Generic::new(File::from(fd), Interest::WRITE, Mode::Level);
    let pipe = handle
        .insert_source(source, move |_, file, _| {
            Ok(match step_write(&mut &**file, &bytes, &mut written) {
                Progress::More => PostAction::Continue,
                Progress::Done => PostAction::Remove,
            })
        })
        .map_err(|error| kind_of(error.error))?;
    let watch = handle.clone();
    handle
        .insert_source(Timer::from_duration(WRITE_DEADLINE), move |_, _, _| {
            // A no-op when the write already ended: the token is versioned.
            watch.remove(pipe);
            TimeoutAction::Drop
        })
        .map(|_| ())
        .map_err(|error| kind_of(error.error))
}

/// A loop error's kind, which is all that is ever printed of it.
fn kind_of(error: calloop::Error) -> ErrorKind {
    std::io::Error::from(error).kind()
}

#[derive(Debug, PartialEq, Eq)]
enum Progress {
    More,
    Done,
}

/// At most one pipe buffer's page (the pipe is writable, so it has room for one: this does
/// not block).
fn step_write(pipe: &mut impl Write, bytes: &[u8], written: &mut usize) -> Progress {
    const PAGE: usize = 4096;
    let rest = bytes.get(*written..).unwrap_or_default();
    if rest.is_empty() {
        return Progress::Done;
    }
    match pipe.write(&rest[..rest.len().min(PAGE)]) {
        Ok(0) => Progress::Done,
        Ok(n) => {
            *written += n;
            if *written >= bytes.len() {
                Progress::Done
            } else {
                Progress::More
            }
        }
        Err(error) if error.kind() == ErrorKind::Interrupted => Progress::More,
        Err(_) => Progress::Done,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, ErrorKind};

    use super::super::content::ReadLimit;
    use super::{Progress, ReadEnd, step_read, step_write};

    /// A read's steps until it ended, and how (sizes only).
    type Ended = (usize, Result<usize, &'static str>);

    fn read_all(input: &[u8], limit: u64) -> Ended {
        let mut pipe = Cursor::new(input.to_vec());
        let mut bytes = Vec::new();
        for steps in 1..100 {
            if let Some(end) = step_read(&mut pipe, &mut bytes, ReadLimit(limit)) {
                let end = match end {
                    ReadEnd::Whole(b) => Ok(b.len()),
                    ReadEnd::OverLimit => Err("over"),
                    ReadEnd::Broken(_) => Err("broken"),
                };
                return (steps, end);
            }
        }
        (100, Err("never ended"))
    }

    #[test]
    fn a_read_ends_at_the_close_or_past_the_limit() {
        let big = vec![b'x'; (1 << 16) + 5];
        let cases: Vec<(&[u8], u64, Ended)> = vec![
            (b"", 10, (1, Ok(0))),
            (b"made-up", 10, (2, Ok(7))),
            (b"made-up", 7, (2, Ok(7))),
            (b"made-up", 6, (1, Err("over"))),
            (&big, 1 << 20, (3, Ok((1 << 16) + 5))),
        ];
        for (input, limit, want) in cases {
            assert_eq!(
                read_all(input, limit),
                want,
                "{} bytes, limit {limit}",
                input.len()
            );
        }
    }

    /// A writer that takes at most `room` bytes a call, or fails.
    struct Narrow {
        taken: Vec<u8>,
        room: usize,
        fail: Option<ErrorKind>,
    }

    impl std::io::Write for Narrow {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if let Some(kind) = self.fail {
                return Err(kind.into());
            }
            let n = buf.len().min(self.room);
            self.taken.extend_from_slice(&buf[..n]);
            Ok(n)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_write_goes_a_page_at_a_time_and_stops_when_the_reader_goes() {
        let bytes = vec![7_u8; 10_000];
        let cases = [
            (usize::MAX, None, 3, 10_000),
            (1000, None, 10, 10_000),
            (1000, Some(ErrorKind::BrokenPipe), 1, 0),
        ];
        for (room, fail, want_steps, want_taken) in cases {
            let mut pipe = Narrow {
                taken: Vec::new(),
                room,
                fail,
            };
            let mut written = 0;
            let steps = (1..=100)
                .find(|_| step_write(&mut pipe, &bytes, &mut written) == Progress::Done)
                .unwrap_or(0);
            assert_eq!(
                (steps, pipe.taken.len()),
                (want_steps, want_taken),
                "{room} {fail:?}"
            );
        }
    }
}
