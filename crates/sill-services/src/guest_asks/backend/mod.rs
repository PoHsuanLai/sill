//! The service's effects: inferd's bus, the notification and the notifications server's answers.

mod links;
mod notice;
mod run;
mod wire;

pub(crate) use run::run;

pub use links::GuestAsksLinks;
