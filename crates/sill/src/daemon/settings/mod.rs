//! The settings the daemon holds and how they reach the surfaces: the store of the file, the
//! watch that publishes each settled change, the focus task that writes the mode back.

mod appearance;
mod focus;
mod publish;
mod store;
mod styles;
mod watch;
mod write_key;

pub use appearance::{
    appearance_store, load_appearance, publish_watched_appearance, spawn_appearance_watch,
};
pub use focus::spawn_focus;
pub use publish::{publish_watched, reload};
#[cfg(test)]
pub use store::save;
pub use store::{desktop_store, load, store};
pub use styles::{StyleKeeper, spawn_turn_on_follower};
pub use watch::spawn_watch;
pub use write_key::write_key;
