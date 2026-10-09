//! Dev-only helpers shared by every crate's tests: a private `dbus-daemon`, scratch folders,
//! signal listeners, and (in `examples/`) the stand-in programs tests and scripts run.

mod modules;
mod private_bus;
mod signals;
mod temp_dir;

pub use modules::assert_all_modules_included;
pub use private_bus::PrivateBus;
pub use signals::{Heard, Signals};
pub use temp_dir::TempDir;
