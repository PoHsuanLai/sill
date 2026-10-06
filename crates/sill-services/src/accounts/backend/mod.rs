//! The service's effects: accountd's bus, the notice and the notifications server's answers.

mod actions;
mod links;
mod notice;
mod run;
mod wire;

pub(crate) use run::run;

pub use links::AccountsLinks;
