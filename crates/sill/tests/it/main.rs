//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason (CONVENTIONS.md, Tests).

mod cosmic_seeds;
mod host_signals;
mod ipc;
mod latency;
mod lint;
mod voice_consent;
// The voice bench is sill-services' own test support, shared by path (its macro names `crate::voice_support`).
#[path = "../../../sill-services/tests/it/voice_support/mod.rs"]
mod voice_support;
