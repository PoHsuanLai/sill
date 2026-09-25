//! The control center (design/20 §1.5, design/13 §13.3.7): a popover from its bar status item,
//! drawn in the bar's one reusable popup (so it takes part in the bar's hover switch: it is
//! `BarMenu::ControlCenter`, its body `PopupBody::ControlCenter`), 320 wide, a two-column grid
//! of module tiles (`control_center.*`), sliders spanning both columns, capped at
//! `output_h - bar_h - control_center.bottom_margin_px` and scrolling past that.
//!
//! Modules, in `control_center.modules` order: Wi-Fi, Bluetooth, Focus, Display (brightness),
//! Sound (volume and the output's name), Now Playing, Appearance (quire's `AppearancePicker`),
//! Battery (level, time left, and the power profile's `SegmentedControl`, which has no module
//! of its own in design/22's list; FINDINGS "M5 freeze"). The Wi-Fi and Bluetooth tiles'
//! chevrons open their detail pages in place, sliding (`slide-l` in, `slide-r` back; [`page`]).
//!
//! Only this module's own files and the bar files named in FINDINGS "M5 freeze" belong to the
//! control-center agent; every service it reads is frozen in `sill_services` and every hook in
//! `crate::hooks`.

pub mod geometry;
pub mod labels;
pub mod link;
pub mod modules;
pub mod page;
pub mod pages;

mod panel;

pub use geometry::{Placed, Span, estimate_size, height_cap, place, span};
pub use labels::{
    bluetooth_status, device_detail, focus_status, network_detail, profile_label, track_lines,
    wifi_status,
};
pub use link::{ControlCenterAsk, ControlCenterLink};
pub use page::{CcPage, Detail, PageInput, PageOut, Slide, step_page};

use dioxus::prelude::*;
use sill_services::OutputName;

/// The control center's contents, inside the bar popup's `Ds` root. `output` is the bar's
/// (the Display module's brightness target). Escape on the module page, or a pick that ends the
/// panel, calls `on_close`; every page change calls `on_page` so the popup can resize to
/// [`estimate_size`] (shell-host's content fit does not measure, G7).
///
/// Needs `sill_services::Services`, `sill_settings::SettingsFeed` and `sill_settings::FocusLink`
/// in the popup root's context: the bar's popup root does not have them before M5 agent (c)
/// adds them (FINDINGS "M5 freeze").
#[component]
pub fn ControlCenter(
    output: Option<OutputName>,
    on_close: EventHandler<()>,
    on_page: EventHandler<CcPage>,
) -> Element {
    panel::control_center(output, on_close, on_page)
}
