//! The DDC worker: one thread owns every bus ([`worker::run`]), reads with retries and writes
//! verified ([`retry`]).

pub(crate) mod retry;
pub(crate) mod worker;
