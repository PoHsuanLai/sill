//! The app switcher (design/13 §13.3.5, design/20 §1.11): Cmd+Tab over applications, not windows.
//! COSMIC's `WindowSwitcher` / `WindowSwitcherPrevious` shortcuts run `sill switcher next|prev`,
//! which the daemon writes to the [`SwitcherLink`]; the switcher surface on the chosen output takes
//! the keyboard (`Exclusive`) at once, so it sees the chord's modifier go up (Alt's or Super's
//! key-up, or shell-host's modifiers on focus: `hold.rs`, `watch.rs`), and runs the pure
//! [`machine`]: a quick tap switches to the previous app with no panel, a hold past
//! `switcher.show_delay_ms` fades quire's `AppSwitcher` in (the apps from
//! `sill_services::switcher_apps`), Tab and the arrows move, Q quits and H hides the selected app,
//! Escape cancels, the release activates. The machine's outputs become `WlCommand`s through
//! `sill_services::switcher::{activate, quit, hide}`.
//!
//! One warm, hidden Overlay per output ([`layer::switcher_layer`]), mapped from the chord until
//! it closes, drawing nothing until the panel shows: quire's `AppSwitcher` in `Ds { material:
//! Osd, chrome: Transparent }`, fed by [`tiles`].

pub mod keys;
pub mod layer;
pub mod link;
pub mod machine;

pub mod tiles;

mod drive;
mod hold;
mod icon;
mod panel;
mod session;
mod watch;

pub use keys::{Chord, chord_after_keyup, sw_key};
pub use layer::{PANEL_ID, PANEL_RADIUS, switcher_layer};
pub use link::{AskSerial, SwitcherAsk, SwitcherAsked, SwitcherLink};
pub use machine::{Dir, SwIn, SwKey, SwOut, Switcher, SwitcherParams, step};
pub use tiles::{index_of, key_of, metrics_of, output_width, tiles};

use dioxus::prelude::*;
use shell_host::OutputInfo;

use crate::host_modality::HostModalityBridge;

/// The switcher on `output`. Needs a [`SwitcherLink`] root context beside the daemon's usual
/// ones (the daemon's `surface_root` gives every surface one).
#[component]
pub fn SwitcherPanel(output: OutputInfo) -> Element {
    rsx! {
        HostModalityBridge { {panel::switcher_panel(output)} }
    }
}
