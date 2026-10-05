//! The companion's wire as quire's view types. quire may not name an agent crate, so each wire
//! type that has a view (an effect, a run's state, an answer, a plan, a form, a footer) is
//! converted here, by a total function pinned by a `*_total` test that visits every variant.
//!
//! Words come from the wire's own labelled text or fixed phrases; a handle the companion left in
//! a text is shown through `ViewCx::display` (the router resolves it for the screen with
//! `Session.Display`, never a model), and reads as an ellipsis until it has been.

mod answer;
mod card;
mod footer;
mod form;
mod marks;
mod plan;
#[cfg(test)]
mod tests;
mod words;

use docket_core::Handle;
use jiff::tz::TimeZone;

pub use answer::{answer_view, without_actions};
pub use card::card_action;
pub use footer::footer_view;
pub use form::form_view;
pub use marks::{effect_mark, run_place, run_state};
pub use plan::plan_view;
pub use words::refusal_words;

/// What a conversion reads besides the wire: the person's time zone, and the text a handle
/// stands for where the router has resolved one.
#[allow(missing_debug_implementations)]
pub struct ViewCx<'a> {
    /// Where times are worded.
    pub zone: &'a TimeZone,
    /// The text of a handle, once resolved for the screen.
    pub display: &'a dyn Fn(Handle) -> Option<String>,
}
