//! The service's effects: accountd's bus, the notice and the notifications server's answers.

pub(crate) mod actions;
mod links;
mod notice;
mod retry;
mod run;
pub(crate) mod wire;

pub(crate) use run::run;

pub use links::AccountsLinks;
