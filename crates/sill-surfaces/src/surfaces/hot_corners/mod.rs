//! Hot corners (design/20 §1.16; macOS's Hot Corners): a transparent square of
//! `hot_corners.size_px` in each corner whose action is not `None`, on every output. The pointer
//! resting in one for `hot_corners.dwell_ms` fires it once; it fires again only after the
//! pointer has left and `rearm_ms` has passed ([`machine::step`]). A firing is written to the
//! [`CornerLink`]; the daemon carries out the action (`crates/sill/src/corners.rs`), since
//! opening the launcher, the notification center or the control center, locking, minimising
//! windows and running a command are its links and services, not a surface's.
//!
//! Never painted: the root draws one transparent element that takes the pointer, no `Ds`, no
//! material, no blur; the compositor still delivers pointer enter and leave to it.

pub mod layer;
pub mod link;
pub mod machine;

pub use layer::corner_layer;
pub use link::{CornerHit, CornerLink};
pub use machine::{CornerIn, CornerMachine, CornerOut, CornerParams, step};

use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use shell_host::{OutputId, OutputInfo};
use sill_settings::{CornerAction, HotCornersSettings};

use crate::hooks::use_settings;

/// A screen corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    /// `top-left`: the namespace's word.
    pub fn word(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }
}

/// What `corner` does per `settings`.
pub fn corner_action(settings: &HotCornersSettings, corner: Corner) -> CornerAction {
    match corner {
        Corner::TopLeft => settings.top_left,
        Corner::TopRight => settings.top_right,
        Corner::BottomLeft => settings.bottom_left,
        Corner::BottomRight => settings.bottom_right,
    }
}

/// The command `corner` runs when its action is `Command`.
pub fn corner_command(settings: &HotCornersSettings, corner: Corner) -> &str {
    match corner {
        Corner::TopLeft => &settings.top_left_command,
        Corner::TopRight => &settings.top_right_command,
        Corner::BottomLeft => &settings.bottom_left_command,
        Corner::BottomRight => &settings.bottom_right_command,
    }
}

/// The corners that get a surface: every one whose action is not `None`.
pub fn active_corners(settings: &HotCornersSettings) -> Vec<Corner> {
    Corner::ALL
        .into_iter()
        .filter(|corner| corner_action(settings, *corner) != CornerAction::None)
        .collect()
}

/// This surface's stylesheet: the one rule the transparent element needs (a bare `div` has no
/// height without one), lint-clean under `Profile::Strict` (`tests/hot_corners_render.rs`).
pub const HOT_CORNERS_CSS: &str = include_str!("../../style/hot_corners.css");

/// `params_of`'s own numbers, from `hot_corners.*`.
fn params_of(settings: &HotCornersSettings) -> CornerParams {
    CornerParams {
        dwell_ms: u32::from(settings.dwell_ms.0),
        rearm_ms: u32::from(settings.rearm_ms.0),
    }
}

/// Feed `input` to the machine held in `machine`/`waking`/`params`, applying every effect: `Fire`
/// hits `link`, `StartDwell`/`StartRearm` (re)arm quire's one timer (`ds::sleep`) and feed the
/// elapsed event back in when it wakes (a stale token is simply ignored by `step`, so a timer
/// that outlives its phase is harmless; cancelling it early, as here, just skips that wait).
fn feed(
    mut machine: Signal<CornerMachine>,
    waking: CopyValue<Option<Task>>,
    params: CopyValue<CornerParams>,
    link: CornerLink,
    output: OutputId,
    corner: Corner,
    input: CornerIn,
) {
    let (next, outs) = step(*machine.peek(), input, *params.peek());
    machine.set(next);
    for out in outs {
        match out {
            CornerOut::Fire => link.hit(corner, output),
            CornerOut::StartDwell(token, ms) => start(
                waking,
                machine,
                params,
                link.clone(),
                output,
                corner,
                ms,
                CornerIn::DwellElapsed(token),
            ),
            CornerOut::StartRearm(token, ms) => start(
                waking,
                machine,
                params,
                link.clone(),
                output,
                corner,
                ms,
                CornerIn::RearmElapsed(token),
            ),
        }
    }
}

/// Cancel the running timer (if any) and start a fresh one: sleep `ms`, then [`feed`] `then`.
#[allow(clippy::too_many_arguments)]
fn start(
    mut waking: CopyValue<Option<Task>>,
    machine: Signal<CornerMachine>,
    params: CopyValue<CornerParams>,
    link: CornerLink,
    output: OutputId,
    corner: Corner,
    ms: u32,
    then: CornerIn,
) {
    if let Some(old) = waking.take() {
        old.cancel();
    }
    waking.set(Some(spawn(async move {
        ds::sleep(Duration::from_millis(u64::from(ms))).await;
        feed(machine, waking, params, link, output, corner, then);
    })));
}

/// `corner` of `output`: a transparent element taking the pointer over the whole square (no
/// `Ds`, no material, never painted), feeding [`machine::step`] from its enter, move and leave,
/// quire's timer for the dwell and the re-arm, and [`CornerLink::hit`] on `Fire`.
#[component]
pub fn HotCorner(output: OutputInfo, corner: Corner) -> Element {
    let link = use_context::<CornerLink>();
    let file = use_settings()();
    let now_params = params_of(&file.hot_corners);
    let machine = use_signal(CornerMachine::default);
    let waking = use_hook(|| CopyValue::new(None::<Task>));
    let mut params = use_hook(|| CopyValue::new(now_params));
    if *params.peek() != now_params {
        params.set(now_params);
    }
    let output_id = output.id;

    let enter_link = link.clone();
    let leave_link = link;

    rsx! {
        style { {HOT_CORNERS_CSS} }
        div {
            class: "sill-hot-corner",
            onpointerenter: move |_| {
                feed(machine, waking, params, enter_link.clone(), output_id, corner, CornerIn::Enter);
            },
            onpointerleave: move |_| {
                feed(machine, waking, params, leave_link.clone(), output_id, corner, CornerIn::Leave);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use sill_settings::{CornerAction, HotCornersSettings};

    use super::{Corner, active_corners, corner_action, corner_command};

    #[test]
    fn only_corners_with_an_action_get_a_surface() {
        let mut settings = HotCornersSettings::default();
        assert_eq!(active_corners(&settings), Vec::new());
        settings.top_left = CornerAction::Launcher;
        settings.bottom_right = CornerAction::Command;
        settings.bottom_right_command = "foot".to_owned();
        assert_eq!(
            active_corners(&settings),
            vec![Corner::TopLeft, Corner::BottomRight]
        );
        assert_eq!(
            corner_action(&settings, Corner::TopLeft),
            CornerAction::Launcher
        );
        assert_eq!(corner_command(&settings, Corner::BottomRight), "foot");
        assert_eq!(corner_command(&settings, Corner::TopRight), "");
    }
}
