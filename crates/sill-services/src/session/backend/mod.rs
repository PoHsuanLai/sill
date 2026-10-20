//! The service's effects: what it reaches outside the process.

mod ending;
mod live;

pub(crate) use live::run;
