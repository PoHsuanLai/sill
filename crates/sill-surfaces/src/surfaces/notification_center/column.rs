//! The column and its catcher. While the center is drawn: the transparent catcher under
//! everything (a press on it closes the center, as a click beside macOS's Notification Center
//! does) and the column at the right edge, `notifications.center_width_px` wide, painted by a
//! quire `Surface` in the Popover material. The column plays quire's entrance as it mounts (keyed
//! by the opening) and quire's exit while leaving (FINDINGS Q123: quire has no edge panel yet).

use dioxus::prelude::*;
use ds::{Material, Surface};
use shell_host::OutputId;

use super::contents::Contents;
use super::drive::{ENTRANCE, EXIT, use_center_ctx, use_phase};
use super::layer::PANEL_ID;
use super::phase::Phase;
use crate::hooks::use_settings;

/// The column on `output`, while the center is drawn there.
#[component]
pub(super) fn Column(output: OutputId) -> Element {
    let phase = use_phase(output)();
    let width = use_settings()().notifications.center_width_px.0;
    let (motion, look) = match phase {
        Phase::Leaving(_) => (EXIT.class(), "leaving"),
        Phase::Hidden(_) | Phase::Up(_) => (ENTRANCE.class(), "up"),
    };
    let Some(opening) = phase.drawn() else {
        return rsx! {};
    };
    rsx! {
        Opened {
            key: "{opening.0}",
            motion,
            look,
            width,
        }
    }
}

/// One opening's catcher and column: its own handlers' state, so each opening starts closed.
#[component]
fn Opened(motion: &'static str, look: &'static str, width: u16) -> Element {
    let ctx = use_center_ctx();
    use_context_provider(|| ctx.clone());
    let close = ctx.clone();
    rsx! {
        div {
            class: "sill-nc-catcher",
            onpointerdown: move |_| close.close(),
        }
        div {
            id: PANEL_ID,
            class: "sill-nc-panel {motion}",
            "data-pulse": "a",
            "data-look": look,
            style: "width:{width}px",
            Surface { material: Material::Popover,
                Contents {}
            }
        }
    }
}
