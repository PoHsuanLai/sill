//! The sill IPC protocol: newline-delimited JSON requests and responses, one request per
//! connection.
//!
//! Pure: no socket, no runtime. The daemon (`sill::ipc`) and the command line (`sill::cli`) own
//! the stream; this crate owns the words on it. A [`request::Request`] is one of seven groups, each with
//! its own file: [`show`] opens and closes panels, [`status`] reads a state, [`act`] changes
//! something, [`debug`] probes. What a status answers is a `sill-model` type, serialised.

pub mod act;
pub mod debug;
pub mod error;
pub mod line;
pub mod request;
pub mod response;
pub mod show;
pub mod status;
