//! The service's effects: what it reaches outside the process.

mod backdrop;
mod config;
mod resize;
mod run;

pub(crate) use run::{Places, cache_directory, run};
