//! The launcher's search: the runner that queries every provider off the UI thread and merges
//! their batches ([`ResultsService`]), its history and frecency, the providers themselves, and
//! the previews a row names.
//!
//! This crate never reaches `dioxus` (`scripts/check-boundary.sh`): the launcher reads the
//! results through `use_service::<ResultsService>()`.

pub mod preview;
pub mod providers;
pub mod results;

pub use results::ResultsService;
