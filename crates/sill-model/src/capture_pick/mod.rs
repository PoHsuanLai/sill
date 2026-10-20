//! The screen-sharing picker as the shell holds it: what an app asked to share (`Ask`), what
//! there is to share (`Candidates`), the choices that follow from both (`plan`), what the
//! person has picked (`Picks`) and the answer that goes back (`reply`). The transom portal
//! backend asks on `org.quire.Shell1.PickCaptureSource`; `sill-services` serves it and
//! `sill-overlays` draws the sheet. Everything here is pure and table-tested.
//!
//! One request is open at a time, of any kind (sources, a colour or a permission question): a
//! second while one is open is refused ([`admits`]). The colour request (`color`) is answered with a cancel until a surface
//! draws the eyedropper (`eyedrop`: the picture it points into, the pixel under the pointer, the
//! magnifier). The permission question (`permission`: `org.quire.Shell1.AskPermission`, the portal's
//! yes-or-no question about an app) is answered by a button of its sheet. The compositor asks the
//! same way when a program wants a power (`consent`, with every word in `consent_words`): its
//! questions and the portal's share one line, one sheet at a time, the rest waiting in order.

mod ask;
mod color;
mod consent;
mod consent_words;
mod eyedrop;
mod from_desktop;
mod permission;
mod plan;
mod reply;
mod step;
#[cfg(test)]
mod tests;

pub use ask::{Ask, AskError, Kinds, Plurality, Pointer, Remembered};
pub use color::{ColorAsk, ColorAsking, ColorOutcome, ColorReply, Eyedropper, Rgb8};
pub use consent::consent_ask;
pub use eyedrop::{
    CELL, EyedropIn, EyedropOut, Frozen, GRID, Loupe, SIDE, ShotPixel, colour_at, eyedrop_step,
    loupe_at, pick_at, pixel_under,
};
pub use from_desktop::windows_of;
pub use permission::{AskedBy, PermissionAsk, PermissionAsking, PermissionOutcome};
pub use plan::{Candidates, Choice, OutputCandidate, Plan, Source, Tab, WindowCandidate, plan};
pub use reply::{Picks, Reply, can_allow, reply, toggle};
pub use step::{
    Admit, AskId, Asking, CaptureCommand, CaptureEvent, CaptureLink, CaptureOut, CapturePicker,
    Outcome, WAITING_MAX, admits, holds_consent, permission_admits, step,
};
