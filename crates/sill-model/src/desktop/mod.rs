//! The desktop as the shell sees it, whichever compositor protocol carries it: which backend
//! the compositor's globals select, the views of its apps, windows and Spaces, the commands
//! the shell sends and whether a backend can carry each, and the notes the compositor's shell
//! protocol sends the shell. Pure; `sill-services` fills the views
//! from the wire.

mod backend;
mod command;
mod model;
mod note;
pub mod target;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;

pub use backend::{DesktopBackend, GlobalOffer};
pub use casement_protocols::vocab::{Corner, KeyGesture, ShellRole, WindowEntry, WireCode};
pub use command::{DesktopCommand, Supports, supports};
pub use model::{ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView};
pub use note::{OverviewNote, ShellNote, SwitcherNote};
