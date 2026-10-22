//! Hiding and showing the menu bar (`bar.autohide`): the Mac's "Automatically hide and show the
//! menu bar". Pure: the phase machine is `sill_ui_kit::autohide`'s, shared with the dock; this
//! file is the bar's half of it (which modes hide on the desktop, what holds the bar shown, which
//! regions it takes) and the effects the runtime applies (`hide.rs`).
//!
//! On the desktop `Always` and `DesktopOnly` start `Hidden`: no exclusive zone, the input region
//! is a thin strip at the top edge, and the pointer resting there (`dock.autohide_delay_ms`)
//! slides the bar down over the windows. An open popup holds it shown; after the last one closes
//! with the pointer off the bar it slides away. In full screen the compositor shows and hides the
//! bar itself (`Edge`): this machine only makes the content drawn and hit-testable, with no slide.
//! `DesktopOnly` rests fixed on an output showing a full-screen Space ([`mode_on`]).

use ds::base::time::stamp::{Ms, Stamp};
use ds::prelude::Motion;
use ds::style::tokens::easing::EasingToken;
use shell_host::{BlurSpec, ElementId, RegionSpec};
use sill_model::desktop::Shown;
use sill_model::units::Cpx;
use sill_settings::{MenuBarHiding, ShellFile};
use sill_ui_kit::autohide::{self, Change, Hold, Input, Phase, Timing, advance};

/// The element the bar's content is drawn in; it slides, and the shown regions follow it.
pub(super) const BODY_ID: &str = "bar-body";
/// The element at the top edge that takes the pointer while the bar is hidden.
pub(super) const STRIP_ID: &str = "bar-strip";

/// Where the pointer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Pointer {
    #[default]
    Outside,
    Inside,
}

/// The bar's whole hiding state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct BarReveal {
    pub phase: Phase,
    pub pointer: Pointer,
}

impl BarReveal {
    /// The bar at rest under `mode`.
    pub(super) fn at_rest(mode: MenuBarHiding) -> BarReveal {
        BarReveal {
            phase: rest_phase(mode),
            pointer: Pointer::Outside,
        }
    }
}

/// The phase the desktop gives `mode`: the two that hide start hidden, the others are fixed.
pub(super) fn rest_phase(mode: MenuBarHiding) -> Phase {
    match mode {
        MenuBarHiding::Always | MenuBarHiding::DesktopOnly => Phase::Hidden,
        MenuBarHiding::FullScreenOnly | MenuBarHiding::Never => Phase::Fixed,
    }
}

/// The mode the bar rests under on an output: `DesktopOnly` hides on the desktop but stays up over
/// a full-screen Space, as `Never` does (the compositor then keeps it and sizes the window below
/// it); the other modes are the same everywhere.
pub(super) fn mode_on(mode: MenuBarHiding, full_screen: Shown) -> MenuBarHiding {
    match (mode, full_screen) {
        (MenuBarHiding::DesktopOnly, Shown::Shown) => MenuBarHiding::Never,
        (mode, _) => mode,
    }
}

/// One input to the bar's reveal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RevealEvent {
    /// The pointer entered the bar (or its strip).
    Enter,
    /// The pointer left the bar.
    Leave,
    /// A popup of the bar opened (a toggle over IPC opens one with the bar hidden).
    MenuOpened,
    /// A popup of the bar closed.
    MenuClosed,
    /// The one-shot timer of [`RevealEffect::WakeAt`] fired.
    Tick,
    /// A frame callback.
    Frame,
    /// The compositor shows or hides the bar over a full-screen window.
    Edge(Shown),
}

/// One thing the runtime must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RevealEffect {
    /// Draw this frame.
    Redraw,
    RequestFrames,
    StopFrames,
    /// Send [`RevealEvent::Tick`] at this instant.
    WakeAt(Stamp),
    SetInput(RegionSpec),
    SetBlur(BlurSpec),
}

/// The auto-hide numbers: the dock's delay, slide time and curves (`dock.autohide_*`), and no
/// slide at all when the person asked for reduced motion.
pub(super) fn timing(file: &ShellFile, motion: Motion) -> Timing {
    let dock = &file.dock;
    Timing {
        delay: Ms::from(dock.autohide_delay_ms.0),
        slide_ms: match motion {
            Motion::Standard => u64::from(dock.autohide_slide_ms.0),
            Motion::Reduced => 0,
        },
        ease_out: EasingToken::Out.easing().curve(),
        ease_exit: EasingToken::Exit.easing().curve(),
    }
}

/// How far above its place a hidden bar sits: its own height.
pub(super) fn slide_distance(file: &ShellFile) -> Cpx {
    Cpx::px(i32::from(file.bar.height_px.0))
}

/// How far up the bar is drawn at `now`.
pub(super) fn lifted(state: &BarReveal, now: Stamp, t: &Timing, file: &ShellFile) -> Cpx {
    autohide::slide_offset(&state.phase, now, t, slide_distance(file))
}

/// The input region while the bar is shown by the machine: its content.
pub(super) fn shown_input() -> RegionSpec {
    element(BODY_ID)
}

/// The blur region while the bar is shown by the machine: its content, wherever it is sliding.
pub(super) fn shown_blur() -> BlurSpec {
    BlurSpec::Region(element(BODY_ID))
}

/// The input region while the bar is hidden: the strip at the top edge.
pub(super) fn strip_input() -> RegionSpec {
    element(STRIP_ID)
}

fn element(id: &str) -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(id),
        radius: 0,
        inflate: 0,
    }
}

/// `state` after `event` at `now` while something does or does not `hold` the bar shown, and
/// what the runtime must do.
pub(super) fn step(
    state: BarReveal,
    event: RevealEvent,
    now: Stamp,
    hold: Hold,
    t: &Timing,
) -> (BarReveal, Vec<RevealEffect>) {
    let pointer = match event {
        RevealEvent::Enter => Pointer::Inside,
        RevealEvent::Leave => Pointer::Outside,
        _ => state.pointer,
    };
    let input = match event {
        RevealEvent::Enter => Some(Input::Inside),
        RevealEvent::Leave => Some(Input::Outside),
        RevealEvent::MenuClosed if pointer == Pointer::Outside => Some(Input::Outside),
        RevealEvent::MenuClosed => None,
        RevealEvent::MenuOpened => Some(Input::Edge(Shown::Shown)),
        RevealEvent::Tick => Some(Input::Tick),
        RevealEvent::Frame => Some(Input::Frame),
        RevealEvent::Edge(shown) => Some(Input::Edge(shown)),
    };
    let (phase, change) = match input {
        Some(input) => advance(state.phase, input, now, hold, t),
        None => (state.phase, None),
    };
    let mut out = Vec::new();
    if let Some(change) = change {
        effects(change, &mut out);
    }
    if event == RevealEvent::Frame {
        out.push(if autohide::animating(&phase) {
            RevealEffect::Redraw
        } else {
            RevealEffect::StopFrames
        });
    }
    (BarReveal { phase, pointer }, out)
}

/// What the bar's layer does for `change`.
fn effects(change: Change, out: &mut Vec<RevealEffect>) {
    let shown = |out: &mut Vec<RevealEffect>| {
        out.push(RevealEffect::SetInput(shown_input()));
        out.push(RevealEffect::SetBlur(shown_blur()));
    };
    match change {
        Change::Arm(at) => out.push(RevealEffect::WakeAt(at)),
        Change::Show => {
            shown(out);
            out.push(RevealEffect::RequestFrames);
        }
        Change::Pop => shown(out),
        Change::Hide => out.push(RevealEffect::RequestFrames),
        Change::Rest | Change::Tuck => {
            out.push(RevealEffect::SetInput(strip_input()));
            out.push(RevealEffect::SetBlur(BlurSpec::None));
        }
    }
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
