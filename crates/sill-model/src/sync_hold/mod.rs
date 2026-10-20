//! The mass deletions syncd holds back, as the shell tells the person: a notification per held
//! dataset whose one action opens Settings at the account, withdrawn when the hold ends. The
//! shell never confirms a discard itself; Settings does (porter FINDINGS rel-6b, rel-6c).

mod command;
mod model;
mod notice;
mod step;
#[cfg(test)]
mod tests;

pub use command::{SyncHoldCommand, SyncHoldEffect, SyncHoldEvent};
pub use model::{Dataset, Hold, SyncHoldState};
pub use notice::{NOTICE_ENTRY, Notice, dataset_of, notice_text, open_key, shown_for};
pub use step::step;
