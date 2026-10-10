//! The desktop as the shell sees it, whichever compositor protocol carries it: which backend
//! the compositor's globals select, the views of its apps, windows and Spaces, the commands
//! the shell sends and whether a backend can carry each. Pure; `sill-services` fills the views
//! from the wire.

mod backend;
mod command;
mod model;
#[cfg(test)]
mod tests;

pub use backend::{DesktopBackend, GlobalOffer, backend_for};
pub use command::{DesktopCommand, Supports, supports};
pub use model::{ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView};
