//! What a service can fail at. Starting a service never fails the shell: a missing bus or
//! protocol leaves that service's receiver at its default, and the surface draws the absence.

use std::path::PathBuf;

/// A service start, command or file operation that did not happen.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// The service's task has stopped; its commands go nowhere.
    #[error("the service has stopped")]
    Gone,
    /// The session bus could not be reached or refused a call.
    #[error("d-bus: {0}")]
    Bus(#[from] zbus::Error),
    /// The bus a call needs is absent by choice ([`BusRoute::Absent`](crate::env::BusRoute)).
    #[error("no {0:?} bus")]
    NoBus(crate::env::BusKind),
    /// A file could not be read, written or watched.
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A directory watch could not be started.
    #[error("watch: {0}")]
    Watch(#[from] notify::Error),
    /// The compositor lacks a protocol this service needs: the service stays at its default.
    #[error("the compositor does not offer {0}")]
    Unsupported(&'static str),
    /// The compositor's output management refused or did not answer a change.
    #[error("outputs: {0}")]
    Outputs(String),
    /// A service's own thread could not be started.
    #[error("no thread: {0}")]
    Thread(std::io::Error),
    /// A peer did not answer within its deadline ([`crate::deadline`]); it is named.
    #[error("{0} did not answer in time")]
    TimedOut(&'static str),
    /// A program to launch could not be started.
    #[error("launch: {0}")]
    Launch(String),
}
