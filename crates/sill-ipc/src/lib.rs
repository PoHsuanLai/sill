//! The sill IPC protocol: newline-delimited JSON requests and responses, one request per
//! connection.
//!
//! Pure: no socket, no runtime. The daemon (`sill::ipc`) and the command line (`sill::cli`) own
//! the stream; this crate owns the words on it. A [`request::Request`] is one flat enum; the words
//! some of its variants carry have a file each: [`show`] (panels, the companion), [`act`] (the
//! display keys), [`status`] (what is read), [`debug`] (probes). What a status answers is a
//! `sill-model` type, serialised.

pub mod act;
pub mod debug;
pub mod error;
pub mod line;
pub mod request;
pub mod response;
pub mod show;
pub mod status;
