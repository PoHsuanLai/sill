//! What a fresh account starts with: the dock and the launcher's Recent row hold the desktop's
//! own apps that are installed, in the Mac's order, once. A person who empties either keeps it
//! empty, because the decision looks at whether the store was ever written, not at what it
//! holds.

mod first_apps;
#[cfg(test)]
mod tests;

pub use first_apps::{Store, first_apps, first_dock, first_frecency};
