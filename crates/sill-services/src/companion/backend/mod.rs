//! The service's effects: the session bus, as the shell.

mod answer;
mod cua;
mod intents;
mod memory;
mod probe;
mod run;
mod shown;
mod tell;
mod text;

pub(crate) use run::{Reach, run};
