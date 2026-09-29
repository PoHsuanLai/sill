//! The daemon's IPC: the socket server ([`server`]), the answer to each request ([`dispatch`]) and
//! the launcher toggle's latency line ([`latency`]).

pub mod dispatch;
pub mod latency;
pub mod server;
