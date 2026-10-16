//! The service's effects: what it reaches outside the process.

mod live;
mod reads;

pub(crate) use live::run;
