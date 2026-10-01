//! The service's effects: the socket thread, systemd, the config file and the notices.

mod config;
mod links;
mod notice;
mod run;
mod socket;
mod systemd;

pub(crate) use links::Reach;
pub(crate) use run::run;

pub use links::KeycapLinks;
