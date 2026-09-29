//! The power menu (design/20 §1.8): one Overlay surface per output, kept warm and hidden like
//! the launcher, that shows a full-output quire `Scrim` under a centred `Sheet` holding Log Out,
//! Restart, Shut Down and Suspend as quire `Button`s (`Primary` for the focused one, `Danger`
//! for Shut Down and Restart, `Secondary` otherwise) with `Kbd` hints.
//! Keyboard `Exclusive`: arrows move, Enter confirms, Escape closes; a scrim click closes.
//!
//! Opened by `sill power-menu {open,toggle}`, the bar's app menu ("Shut Down…"), and the
//! launcher's Log Out, Restart and Shut Down rows (focused on that row's action). All of them
//! write one shared [`PowerMenuLink`] (`presence.rs`), which every output's surface follows onto
//! its own visibility; the daemon also feeds it the surface's keyboard focus (a focus loss after
//! holding focus closes it, as the launcher's does). A chosen action goes to
//! `sill_model::session::SessionCommand::Run`.

pub mod backend;
pub mod keymap;
pub mod keys;
pub mod labels;
pub mod layer;
pub mod presence;
pub mod script;
pub mod sheet;

mod drive;
mod follow;
mod panel;

pub use backend::{BackendKind, DryRunSession, PowerMenuBackend, SessionBackend, dry_run_line};
pub use keys::{PowerMenuKey, PowerMenuOut, PowerMenuState, initial, step};
pub use labels::{action_label, button_variant};
pub use layer::{PANEL_BLUR_RADIUS, PANEL_ID, panel_region, power_menu_layer};
pub use presence::{PowerMenuLink, PowerMenuOpening, PowerMenuShown, Preselect};
pub use script::{PowerMenuScript, ScriptedKey, script_key};

use dioxus::prelude::*;
use shell_host::OutputInfo;

use crate::host_modality::HostModalityBridge;

/// The power menu on `output`. Needs a [`PowerMenuLink`] root context beside the daemon's usual
/// ones (the daemon's `surface_root` gives every surface one).
#[component]
pub fn PowerMenu(output: OutputInfo) -> Element {
    rsx! {
        HostModalityBridge { {panel::power_menu(output)} }
    }
}
