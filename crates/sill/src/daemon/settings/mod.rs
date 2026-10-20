//! The settings the daemon holds and how they reach the surfaces: the store of the file, the
//! watch that publishes each settled change, the focus task that writes the mode back.

mod focus;
mod patch;
mod publish;
mod store;
mod user_style;
mod watch;

pub use focus::spawn_focus;
pub use patch::{KeyEdit, patch};
pub use publish::{publish_watched, reload};
#[cfg(test)]
pub use store::save;
pub use store::{load, store};
pub use user_style::{
    load_user_style, publish_watched_style, reload_user_style, spawn_user_style_watch,
};
pub use watch::spawn_watch;
