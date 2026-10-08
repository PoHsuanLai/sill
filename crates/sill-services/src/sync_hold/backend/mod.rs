//! The service's effects: syncd's bus, the notification and the notifications server's answers.

mod links;
mod notice;
mod run;
mod wire;

pub(crate) use run::run;

pub use links::SyncHoldLinks;
