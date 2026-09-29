//! The launcher's search as state: the merged, sectioned result list, what it can be told, the
//! launcher's policy, and how rows are arranged. The runner and its providers are `sill-search`'s.

mod arrange;
mod command;
mod model;
mod policy;
mod step;
#[cfg(test)]
mod tests;

pub use arrange::{Asked, Typed, arranged};
pub use command::{ResultsEvent, SearchCommand};
pub use model::Results;
pub use policy::{CurrencyPolicy, FilesChoice, LauncherPolicy, WebPolicy};
pub use step::step;
