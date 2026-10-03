//! The children a service starts and does not wait for: the file opener, the trash, whatever a
//! row's default handler is. The daemon starts real processes; a test records the command it
//! would have run, so no test opens a file in the person's session.

use std::ffi::OsString;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};

/// Which process group a child is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessGroup {
    /// The child leads a new group, so it outlives its parent's signals.
    Own,
    /// The parent's own group.
    Shared,
}

/// One child to start: the command line and the activation token the child is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    /// The program, then its arguments.
    pub argv: Vec<OsString>,
    /// Set in `XDG_ACTIVATION_TOKEN` and `DESKTOP_STARTUP_ID` so the child can take focus.
    pub token: Option<String>,
    pub group: ProcessGroup,
}

/// Starts children that nobody waits on.
pub trait Children: Send + Sync + fmt::Debug {
    /// Start `start`, reaping it in the background; the error is why it could not start (the
    /// program missing from `PATH` is `ErrorKind::NotFound`).
    fn detach(&self, start: &Start) -> io::Result<()>;
}

/// The real ones: a process started on the tokio runtime and reaped there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RealChildren;

impl Children for RealChildren {
    fn detach(&self, start: &Start) -> io::Result<()> {
        let (program, args) = start
            .argv
            .split_first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty argument vector"))?;
        let mut command = tokio::process::Command::new(program);
        command.args(args).stdin(std::process::Stdio::null());
        if let Some(token) = &start.token {
            command.env("XDG_ACTIVATION_TOKEN", token);
            command.env("DESKTOP_STARTUP_ID", token);
        }
        if start.group == ProcessGroup::Own {
            command.process_group(0);
        }
        let mut child = command.kill_on_drop(false).spawn()?;
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(())
    }
}

/// What [`Env::isolated`](super::Env::isolated) starts children with: none. Each `Start` is kept
/// and answered `Ok`, as a program that started and was never waited on.
#[derive(Debug, Default)]
pub struct RecordedChildren {
    started: Mutex<Vec<Start>>,
}

impl RecordedChildren {
    /// A recorder, shared with the `Env` it is put in.
    pub fn shared() -> Arc<RecordedChildren> {
        Arc::new(RecordedChildren::default())
    }

    /// Everything started so far, in order.
    pub fn started(&self) -> Vec<Start> {
        self.started
            .lock()
            .map(|started| started.clone())
            .unwrap_or_default()
    }
}

impl Children for RecordedChildren {
    fn detach(&self, start: &Start) -> io::Result<()> {
        if let Ok(mut started) = self.started.lock() {
            started.push(start.clone());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Children, ProcessGroup, RecordedChildren, Start};

    #[test]
    fn a_recorder_keeps_what_it_was_asked_to_start_and_starts_nothing() {
        let recorder = RecordedChildren::default();
        let start = Start {
            argv: vec!["sill-no-such-opener".into(), "/h/a.pdf".into()],
            token: Some("t".to_owned()),
            group: ProcessGroup::Own,
        };
        assert!(recorder.detach(&start).is_ok());
        assert_eq!(recorder.started(), [start]);
    }
}
