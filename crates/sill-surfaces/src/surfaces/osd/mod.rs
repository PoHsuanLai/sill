//! The OSD (design/20 §1.7): the indicator that shows a new volume or brightness level, one
//! Overlay surface per output, spawned hidden and kept warm (`set_visibility`, as the
//! launcher's), mapped for a change and unmapped once it has faded.
//!
//! The daemon writes each change to the shared [`OsdLink`] (`link.rs`): what moved, on which
//! outputs, and where the level comes from. Each surface runs its own show, hold and fade
//! machine (`machine.rs`) from it (`drive.rs`) and draws the level it reads from the services
//! (`reading.rs`): the glyph at `IconSize::Bar` and quire's `Slider`, read-only, on the Osd
//! material tinted by the active Space (design/21 §3).
//!
//! Which changes show is the daemon's decision (`sill::osd`): the ones sill itself makes for
//! `sill display volume|brightness` (the keys), never another app's.

pub mod drive;
pub mod layer;
pub mod link;
pub mod machine;
pub mod reading;

mod view;

pub use layer::{CARD_HEIGHT, CARD_ID, CARD_WIDTH, ROOM, osd_layer};
pub use link::{ChangeSerial, Level, LevelSource, OsdChange, OsdKind, OsdLatest, OsdLink};
pub use machine::{OsdInput, OsdLook, OsdPhase, OsdTimes, step};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The indicator on `output`. Needs an [`OsdLink`] root context beside the daemon's usual ones.
#[component]
pub fn Osd(output: OutputInfo) -> Element {
    view::osd(output)
}
