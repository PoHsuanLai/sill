//! The screen-sharing picker as the shell holds it: what an app asked to share (`Ask`), what
//! there is to share (`Candidates`), the choices that follow from both (`plan`), what the
//! person has picked (`Picks`) and the answer that goes back (`reply`). The transom portal
//! backend asks on `org.quire.Shell1.PickCaptureSource`; `sill-services` serves it and
//! `sill-overlays` draws the sheet. Everything here is pure and table-tested.
//!
//! One request is open at a time, of either kind (sources or a colour): a second while one is open
//! is refused ([`admits`]). The colour request (`color`) is answered with a cancel until a surface
//! draws the eyedropper (`eyedrop`: the picture it points into, the pixel under the pointer, the
//! magnifier).

mod ask;
mod color;
mod eyedrop;
mod from_desktop;
mod plan;
mod reply;
mod step;
#[cfg(test)]
mod tests;

pub use ask::{Ask, AskError, Kinds, Plurality, Pointer};
pub use color::{ColorAsk, ColorAsking, ColorOutcome, ColorReply, Eyedropper, Rgb8};
pub use eyedrop::{
    CELL, EyedropIn, EyedropOut, Frozen, GRID, Loupe, SIDE, ShotPixel, colour_at, eyedrop_step,
    loupe_at, pick_at, pixel_under,
};
pub use from_desktop::windows_of;
pub use plan::{Candidates, Choice, OutputCandidate, Plan, Source, Tab, WindowCandidate, plan};
pub use reply::{Picks, Reply, can_allow, reply, toggle};
pub use step::{
    Admit, AskId, Asking, CaptureCommand, CaptureEvent, CaptureLink, CaptureOut, CapturePicker,
    Outcome, admits, step,
};
