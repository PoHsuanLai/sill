//! The service's effects: what it reaches outside the process.

mod live;
mod upower;

pub(crate) use live::run;
