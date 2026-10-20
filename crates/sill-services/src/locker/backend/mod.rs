//! The service's effects: what it reaches outside the process.

mod live;
mod logind;
mod marker;

pub(crate) use live::run;
