//! The service's effects: what it reaches outside the process.

mod live;
mod pump;
mod wake;

pub(crate) use pump::run;
