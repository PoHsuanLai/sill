//! The service's effects: what it reaches outside the process.

mod dbus;
mod open;
mod reach;
mod run;
mod terminal;
mod uri;
mod viewer;

pub use open::{open_argv, reveal_folder};
pub(crate) use reach::Reach;
pub(crate) use run::run;
pub use uri::file_uri;
