//! The effects end of the thumbnail: feed [`step`] and carry out what it returns. The hold is
//! quire's timer (`ds::sleep`, for `screenshot.thumbnail_hold_ms` or the banners' rest after a
//! hover, never a literal), one at a time and a task of the thumbnail's own scope; `Show` maps
//! the surface, `Hide` unmaps it, `Leave` tells the service. While hidden no task runs.

use std::time::Duration;

use dioxus::core::{Runtime, ScopeId, Task, current_scope_id};
use dioxus::prelude::*;
use shell_host::{SurfaceHandle, SurfaceOp, Visibility};
use sill_services::{Control, ShotCommand};

use super::machine::{Pointer, ThumbIn, ThumbOut, ThumbParams, ThumbState, step};

/// Everything the thumbnail's handlers share. Copy: signals and cells only.
#[derive(Clone, Copy)]
pub(super) struct Thumb {
    pub state: Signal<ThumbState>,
    params: CopyValue<ThumbParams>,
    control: CopyValue<Control<ShotCommand>>,
    surface: CopyValue<Option<SurfaceHandle>>,
    hold: CopyValue<Option<Task>>,
    scope: ScopeId,
}

impl Thumb {
    /// Feed the machine `input` now and carry out what it says.
    pub fn input(self, input: ThumbIn) {
        let (next, outs) = step(*self.state.peek(), input, *self.params.peek());
        let mut state = self.state;
        if *state.peek() != next {
            state.set(next);
        }
        for out in outs {
            self.perform(out);
        }
    }

    /// Whether the pointer is on the card.
    pub fn pointer(self) -> Pointer {
        match *self.state.read() {
            ThumbState::Shown { pointer, .. } => pointer,
            ThumbState::Leaving { .. } | ThumbState::Hidden => Pointer::Away,
        }
    }

    fn perform(self, out: ThumbOut) {
        match out {
            ThumbOut::Show(_) => self.visibility(Visibility::Shown),
            ThumbOut::StartHold(token, ms) => {
                self.stop_hold();
                let wait = Duration::from_millis(u64::from(ms));
                let task = Runtime::current().in_scope(self.scope, || {
                    spawn(async move {
                        ds::sleep(wait).await;
                        self.input(ThumbIn::HoldElapsed(token));
                    })
                });
                let mut hold = self.hold;
                hold.set(Some(task));
            }
            ThumbOut::Leave(id, why) => {
                self.stop_hold();
                // A stopped service has nothing to act on: the thumbnail still goes.
                let _ = self.control.peek().send(ShotCommand::Leave(id, why));
            }
            ThumbOut::Hide => {
                self.stop_hold();
                self.visibility(Visibility::Hidden);
            }
        }
    }

    fn stop_hold(self) {
        let mut hold = self.hold;
        if let Some(task) = hold.take() {
            task.cancel();
        }
    }

    /// Map or unmap the surface; nothing without a host (a test, an SSR render).
    fn visibility(self, visibility: Visibility) {
        if let Some(surface) = self.surface.peek().as_ref() {
            surface.apply(SurfaceOp::SetVisibility(visibility));
        }
    }
}

/// The thumbnail's shared state, its timers owned by the calling component's scope.
pub(super) fn use_thumb(params: ThumbParams, control: Control<ShotCommand>) -> Thumb {
    let surface = try_use_context::<SurfaceHandle>();
    let thumb = use_hook(|| Thumb {
        state: Signal::new(ThumbState::Hidden),
        params: CopyValue::new(params),
        control: CopyValue::new(control),
        surface: CopyValue::new(surface),
        hold: CopyValue::new(None),
        scope: current_scope_id(),
    });
    if *thumb.params.peek() != params {
        let mut cell = thumb.params;
        cell.set(params);
    }
    thumb
}
