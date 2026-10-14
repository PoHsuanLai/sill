//! The one way a command prints its answer to stdout. Rust's `println!` panics when the reader
//! has gone (`sill ping | head -c0`: "failed printing to stdout: Broken pipe"); a
//! command line tool should end quietly instead, as the reader asked for no more. [`emit`] writes
//! through any [`Write`] so the closed-pipe case is a unit test over a writer that fails.

use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;

/// How an attempt to print ended.
#[derive(Debug)]
pub enum Emitted {
    /// Everything was written.
    Done,
    /// The reader closed its end (a closed pipe): nothing is wrong, nothing more is wanted.
    ReaderGone,
    /// Writing failed for another reason.
    Failed(io::Error),
}

impl Emitted {
    /// What the command exits with when this was all it had to do: only a real error fails.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Emitted::Done | Emitted::ReaderGone => ExitCode::SUCCESS,
            Emitted::Failed(_) => ExitCode::FAILURE,
        }
    }
}

/// Write `text` to `out` and flush it.
pub fn emit(out: &mut impl Write, text: &str) -> Emitted {
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Emitted::Done,
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Emitted::ReaderGone,
        Err(error) => Emitted::Failed(error),
    }
}

/// Write `text` to the process's stdout; a real failure is reported on stderr.
pub fn print(text: &str) -> Emitted {
    let emitted = emit(&mut io::stdout().lock(), text);
    if let Emitted::Failed(error) = &emitted {
        eprintln!("sill: writing to stdout: {error}");
    }
    emitted
}

#[cfg(test)]
#[path = "stdout_tests.rs"]
mod tests;
