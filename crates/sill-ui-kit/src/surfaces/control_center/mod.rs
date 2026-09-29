//! The control center (design/20 §1.5, design/13 §13.3.7): a popover from its bar status item,
//! drawn in the bar's one reusable popup (so it takes part in the bar's hover switch: it is
//! `BarMenu::ControlCenter`, its body `PopupBody::ControlCenter`), 320 wide, a two-column grid
//! of module tiles (`control_center.*`), sliders spanning both columns, capped at
//! `output_h - bar_h - control_center.bottom_margin_px` and scrolling past that.
//!
//! Modules, in `control_center.modules` order: Wi-Fi, Bluetooth, Focus, Display (brightness),
//! Sound (volume and the output's name), Now Playing, Appearance (quire's `AppearancePicker`),
//! Battery (level, time left, and the power profile's `SegmentedControl`, which has no module
//! of its own in design/22's list). The Wi-Fi and Bluetooth tiles'
//! chevrons open their detail pages in place, sliding (`slide-l` in, `slide-r` back; [`page`]).
//!
//! Wi-Fi, Bluetooth, Sound, Display, Battery and Now Playing can also be bar items of their own
//! (`control_center.menu_bar_*`, macOS's "Show in Menu Bar"; Wi-Fi, Sound and Battery by
//! default), each dropping down the same module component ([`menu_bar`]). The control center's
//! own item is the last status item, left of the clock, and its panel hangs from the output's
//! right edge.

pub mod geometry;
pub mod labels;
pub mod level;
pub mod menu_bar;
pub mod modules;
pub mod page;
pub mod pages;

mod dropdown;
pub(crate) mod fit;
mod panel;
pub(crate) mod row_phase;
pub(crate) mod rows;
mod sizes;

pub use dropdown::ModuleDropdown;
pub(crate) use panel::PanelCap;

pub use geometry::{Placed, estimate_size, height_cap, place};
pub use labels::{
    bluetooth_status, bluetooth_tile_status, device_detail, focus_status, network_detail,
    profile_label, wifi_status,
};
pub use level::LevelModule;
pub use menu_bar::{MENU_BAR_ORDER, in_menu_bar, menu_bar_modules};
pub use modules::Span;
pub use page::{CcPage, Detail, PageInput, PageOut, Slide, step_page};

use dioxus::prelude::*;
use sill_model::ids::OutputName;

/// The control center's contents, inside the bar popup's `Ds` root. `output` is the bar's
/// (the Display module's brightness target). Escape on the module page, or a pick that ends the
/// panel, calls `on_close`; every page change calls `on_page` so the popup can resize to
/// [`estimate_size`] (shell-host's content fit does not measure).
///
/// Needs `crate::services::Services`, `crate::settings::SettingsFeed` and `crate::links::FocusLink`
/// in the popup root's context.
#[component]
pub fn ControlCenter(
    output: Option<OutputName>,
    on_close: EventHandler<()>,
    #[props(default)] on_page: Option<EventHandler<CcPage>>,
) -> Element {
    panel::control_center(output, on_close, on_page)
}
