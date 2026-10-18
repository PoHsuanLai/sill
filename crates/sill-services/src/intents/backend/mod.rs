//! The service's effects: the session bus.

mod control;
pub mod menus;
mod objects;
mod provider;
mod run;
mod serve;
mod shots;
mod windows;

pub use provider::{ManifestFault, manifest};
pub(crate) use run::{Reach, run};
