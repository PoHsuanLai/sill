//! The desktop's own compositor as `desktop_wl`'s Quire arm: apps, windows and Spaces from
//! `quire_desktop_v1` mapped into the shell's [`sill_model::desktop::DesktopView`], the shell's
//! commands as Quire requests, the shell protocol's notes in the shell's words, and the
//! connection the compositor hands the shell.
//!
//! The pieces, lowest first: `handout` (the seam that yields the handed connection),
//! `surfaces` (which registered surface plays which role), `view` and `command` (pure
//! mappings), `notes` (pure), `arm` (a [`arm::QuireArm`] over any `desktop_client::Desktop`:
//! a fake in tests, the wire client on the shell host's connection) and `shell` (the thread
//! that runs the arm on that connection).

mod arm;
mod command;
mod handout;
mod notes;
mod shell;
mod surfaces;
mod view;

/// The host's surface type, for a caller that registers one without naming the Wayland crate.
pub use wayland_client::protocol::wl_surface::WlSurface;

pub use arm::{ArmError, NoteSink, QuireArm};
pub use command::quire_wire_of;
pub use handout::{
    ConnectionRole, FakeHandout, Handout, HandoutError, ZbusHandout, shell_connection,
};
pub use notes::note_of;
pub use shell::{ShellArm, ShellCall, ShellGone, SpawnError, spawn_shell_arm};
pub use surfaces::{ShellSurfaces, role_of_surface};
pub use view::quire_view;
