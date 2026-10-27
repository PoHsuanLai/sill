//! The OSD (design/20 §1.7): the indicator that shows a new
//! volume or brightness level, one Overlay surface per output, spawned hidden and kept warm
//! (`set_visibility`, as the launcher's), mapped for a change and unmapped once quire's `Osd`
//! card says its exit has faded out (`on_hidden`).
//!
//! The daemon writes each change to the shared [`sill_ui_kit::links::OsdLink`] (`links`): what moved, on which
//! outputs, and where the level comes from. Each surface runs its own show-and-hold machine
//! (`step.rs`) from it (`io.rs`), reads the level and title from the services
//! (`reading.rs`), and hands both to quire's `Osd` card (`view.rs`), which owns its own entrance
//! and exit fade and the Osd material tinted by the active Space (design/21 §3).
//!
//! Which changes show is the daemon's decision (`sill::osd`): the ones sill itself makes for
//! `sill display volume|brightness` (the keys), never another app's.

mod io;
mod layer;
mod reading;
mod step;
mod surface;
pub mod tokens;
mod view;

pub use surface::Osd;
pub use view::OSD_CSS;
