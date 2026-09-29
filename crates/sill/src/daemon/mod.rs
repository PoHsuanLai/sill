//! The daemon: `sill run` ([`run`]) takes the single-instance lock, starts the services, the IPC
//! server and the settings watch, connects the host and keeps one row of surfaces per output.
//! `events` handles what the host reports, `surfaces` spawns what an output lacks (`spawn` any
//! `Surface`), `services`
//! starts and instructs the services, `settings` holds the settings and `msg` is the message
//! other threads send the host.

mod events;
pub mod msg;
mod run;
pub(crate) mod services;
pub(crate) mod settings;
pub(crate) mod spawn;
pub(crate) mod surfaces;

pub use run::run;
