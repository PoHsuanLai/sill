//! The service's effects: what it reaches outside the process.

mod links;
mod live;
mod logind;
mod resolve;
mod screensaver;
mod wl;
mod wl_dispatch;
mod wl_state;

pub(crate) use live::run;

pub use links::IdleLinks;
pub(crate) use links::Reach;
