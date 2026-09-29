//! The settings the daemon holds and how they reach the surfaces: the store of the file, the
//! watch that publishes each settled change, the focus task that writes the mode back.

mod focus;
mod publish;
mod store;
mod watch;

pub use focus::spawn_focus;
pub use publish::{publish_watched, reload};
pub use store::{load, save, store};
pub use watch::spawn_watch;
