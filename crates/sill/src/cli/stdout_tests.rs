//! `emit` over writers that succeed, that report a closed pipe and that fail otherwise.

use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;

use super::{Emitted, emit};

/// A writer whose every write fails with `kind`.
struct Failing(ErrorKind);

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(self.0))
    }
}

#[test]
fn text_reaches_the_writer() {
    let mut out = Vec::new();
    assert!(matches!(
        emit(&mut out, "pci-0000:10:00.0\n"),
        Emitted::Done
    ));
    assert_eq!(out, b"pci-0000:10:00.0\n");
}

#[test]
fn a_closed_pipe_ends_quietly_and_succeeds() {
    let emitted = emit(&mut Failing(ErrorKind::BrokenPipe), "device\n");
    assert!(matches!(emitted, Emitted::ReaderGone));
    assert_eq!(emitted.exit_code(), ExitCode::SUCCESS);
}

#[test]
fn any_other_write_error_is_a_failure() {
    let emitted = emit(&mut Failing(ErrorKind::PermissionDenied), "device\n");
    assert!(matches!(emitted, Emitted::Failed(_)));
    assert_eq!(emitted.exit_code(), ExitCode::FAILURE);
}
