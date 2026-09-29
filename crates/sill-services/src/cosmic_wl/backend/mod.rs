//! The `cosmic_wl` thread: its own Wayland connection, the cctk states and handlers, protocol
//! handles turned into keys and descriptions, and the commands carried out.

mod describe;
mod execute;
mod handlers;
mod keys;
mod publish;
mod state;
mod thread;

pub(crate) use publish::Publish;
pub(crate) use thread::run;
