//! The service's effects: the bus object, the name, and the loop.

mod object;
mod run;
mod serve;
mod wire;

pub(super) use run::run;
