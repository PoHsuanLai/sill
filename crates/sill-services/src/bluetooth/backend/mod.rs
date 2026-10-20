//! The service's effects: what it reaches outside the process.

mod devices;
mod pairing;
mod run;

pub(crate) use run::run;
