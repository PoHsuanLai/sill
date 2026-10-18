//! The service's effects: the session bus, as the shell.

mod answer;
mod cua;
mod helpers;
mod intents;
mod memory;
mod pool;
mod probe;
mod routines;
mod run;
mod shown;
mod tell;
mod terminal;
mod text;

pub(crate) use run::{Reach, run};
