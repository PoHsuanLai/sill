//! The service's effects: the bus object, the name, and the loop.

mod callers;
mod objects;
mod run;
mod serve;

pub(super) use run::run;
