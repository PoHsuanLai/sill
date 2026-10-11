//! The screen-sharing picker as the shell holds it: what an app asked to share (`Ask`), what
//! there is to share (`Candidates`), the choices that follow from both (`plan`), what the
//! person has picked (`Picks`) and the answer that goes back (`reply`). The transom portal
//! backend asks on `org.quire.Shell1.PickCaptureSource`; `sill-services` serves it and
//! `sill-overlays` draws the sheet. Everything here is pure and table-tested.
//!
//! One request is open at a time: a second while one is open is refused ([`admits`]).

mod ask;
mod from_desktop;
mod plan;
mod reply;
mod step;
#[cfg(test)]
mod tests;

pub use ask::{Ask, AskError, Kinds, Plurality, Pointer};
pub use from_desktop::windows_of;
pub use plan::{Candidates, Choice, OutputCandidate, Plan, Source, Tab, WindowCandidate, plan};
pub use reply::{Picks, Reply, can_allow, reply, toggle};
pub use step::{
    Admit, AskId, Asking, CaptureCommand, CaptureEvent, CaptureLink, CaptureOut, CapturePicker,
    Outcome, admits, step,
};
