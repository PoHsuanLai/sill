//! The accounts sheet as the shell holds it (design/31 §4.5 and §5.6, PLAN §2.4): the sheets
//! accountd has opened on `org.quire.AccountsSheet1`, the view each shows, and what the person has
//! typed or switched on it so far (the sheet is controlled: the shell owns every value the form
//! shows). The pure step turns what the person does into the `SheetInput` accountd is told, and
//! nothing else: a typed password lives only in the draft, and is gone from it the moment it is
//! sent. `sill-services` serves the bus; `sill-overlays` draws.

mod command;
mod draft;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{AccountsSheetCommand, Admit, SheetAction, SheetEvent, SheetOut, admits};
pub use draft::{CopyMark, FieldDraft, ListKey, SubmitCount};
pub use model::{AccountsSheets, Awaiting, Draft, MAX_OPEN, OpenSheet, SheetHandle, SheetLink};
pub use step::step;
