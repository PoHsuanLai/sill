//! Which keyboard profile the focused window wants: the open `ProfileSource` trait, the sources
//! sill ships, and the pure choice over them. keycap knows profiles, not apps; this is the whole
//! desktop side of that contract (`keycap_protocol::ToDaemon::Focus`).

mod choose;
mod model;
mod shell;
mod source;
#[cfg(test)]
mod tests;

pub use choose::{choose, focused, wanted};
pub use model::{CategoryTable, DeclaredProfile, FocusedWindow, OwnProfiles, UserOverrides};
pub use shell::{Grip, ShellFocus, ShellKeys, under_shell};
pub use source::{ProfileSource, keymap_aware_profile};
