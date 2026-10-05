//! The service's effects: the session bus.

mod control;
mod menus;
mod objects;
mod provider;
mod run;
mod serve;

pub use provider::manifest;
pub(crate) use run::{Reach, run};
