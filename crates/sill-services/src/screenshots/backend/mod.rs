//! The service's effects: what it reaches outside the process.

mod child;
mod folder_watch;
mod live;
mod preview;

pub(crate) use live::run;
