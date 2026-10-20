//! The service's effects: what it reaches outside the process.

mod fcitx;
mod ibus;
mod live;

pub(crate) use live::run;
