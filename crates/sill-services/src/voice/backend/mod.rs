//! The service's effects: the session bus, as the shell.

mod reader;
mod run;
mod signals;

pub(crate) use run::{Reach, run};
