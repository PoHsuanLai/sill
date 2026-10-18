//! The confirmations intentd asks the person: a queue, one on screen at a time, and how an
//! answer, a withdrawal or an expiry moves it. An unanswered question is a dismissal or an
//! expiry, never an allow: the only way to a yes is a [`SheetChoice`] the offer allows.

mod brief;
mod brief_edit;
#[cfg(test)]
mod brief_tests;
mod brief_wire;
#[cfg(test)]
mod crowd_tests;
#[cfg(test)]
mod limit_tests;
mod model;
mod step;
#[cfg(test)]
mod tests;
mod view;
mod words;

pub use brief::{BRIEF_MAX_BYTES, BriefFlag, BriefKind, BriefSpan, BriefView};
pub use brief_edit::{Reach, ScrollSpot, remapped, removed, span_text};
pub use model::{Pending, Reply, Serial, SheetChoice, Sheets, permits};
pub use step::{SheetEffect, SheetEvent, step};
pub use view::{SheetFact, SheetHeld, SheetView, sheet_view};
