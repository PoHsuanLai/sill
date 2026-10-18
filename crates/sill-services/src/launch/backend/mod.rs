//! The service's effects: what it reaches outside the process.

mod clock;
mod dbus;
mod links;
mod notice;
mod open;
mod outcomes;
mod reach;
mod run;
mod terminal;
mod uri;
mod viewer;

pub use clock::monotonic_now;
pub use links::LaunchLinks;
pub use open::{open_argv, reveal_folder};
pub(crate) use reach::Reach;
pub(crate) use run::run;
pub(crate) use terminal::detect as terminal_wrapper;
pub use uri::file_uri;
