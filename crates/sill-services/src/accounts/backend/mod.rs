//! The service's effects: accountd's bus and the notice.

mod links;
mod notice;
mod retry;
mod run;
pub(crate) mod wire;

pub(crate) use run::run;

pub use links::AccountsLinks;
