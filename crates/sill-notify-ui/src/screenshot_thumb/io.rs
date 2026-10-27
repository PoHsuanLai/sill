//! The effects end of the thumbnail: feed [`step`] and carry out what it returns. The hold is
//! quire's timer (`ds::base::time::clock::sleep`, for the thumbnail hold or the banners' rest after a
//! hover, never a literal), one at a time and a task of the thumbnail's own scope; `Show` maps
//! the surface, `Hide` unmaps it, `Leave` tells the service; a drag out is a task of the same
//! scope awaiting shell-host's outcome. `TakeKeyboard` asks the compositor for the keyboard
//! (`Exclusive`, the one mode that takes it without a click), marks the seat's modality as the
//! keyboard's so quire's focus ring shows before the first key, and focuses the picture;
//! `ReleaseKeyboard` sets the mode back to `None`. While hidden no task runs.

use ds::base::time::clock::sleep;
use std::time::Duration;

use dioxus::core::{Runtime, ScopeId, Task, current_scope_id};
use dioxus::prelude::*;
use ds::focus::select::Select;
use ds::focus::selector::{FocusError, focus_by_selector};
use ds::prelude::*;
use shell_host::{KeyboardMode, SurfaceHandle, Visibility};
use sill_model::screenshots::{ShotCommand, ShotId};
use sill_services::Control;

use sill_ui_kit::links::{BannerLink, Transient};

use crate::keyboard::seat_on_keyboard;

use super::drag::{drag_end, file_drag};
use super::ghost::GhostLook;
use super::picture::ThumbPicture;
use super::step::{DragEnd, ThumbExit, ThumbIn, ThumbOut, ThumbParams, ThumbState, step};

/// The card's press target, the element the keyboard lands on (quire's `ShotThumbnail`).
const PICTURE: &str = ".ds-shot-picture";

/// How long a drag out of the card may run before it is taken as lost.
const DRAG_LIMIT: Duration = Duration::from_secs(120);

/// Everything the thumbnail's handlers share. Copy: signals and cells only.
#[derive(Clone, Copy)]
pub(super) struct Thumb {
    pub state: Signal<ThumbState>,
    params: CopyValue<ThumbParams>,
    control: CopyValue<Control<ShotCommand>>,
    link: CopyValue<BannerLink>,
    surface: CopyValue<Option<SurfaceHandle>>,
    signals: CopyValue<Option<HostSignals>>,
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

    /// `exit` has played shot `id` out: the machine hides if that is the exit it was waiting for
    /// (a replaced shot's card or row settles while a newer one shows, and is nothing).
    pub fn settled(self, id: ShotId, exit: ThumbExit) {
        let state = *self.state.peek();
        if matches!(state, ThumbState::Leaving { id: leaving, exit: how, .. } if leaving == id && how == exit)
        {
            self.input(ThumbIn::ExitSettled);
        }
    }

    /// Carry shot `id`'s file out of the surface (quire's card said a press crossed its drag
    /// threshold; the press is still down), its ghost drawn in `look`, and feed the machine the
    /// outcome when the drop or the cancel comes back. Without a host (a test, an SSR render)
    /// the drag is refused at once.
    pub fn drag_out(self, id: ShotId, picture: &ThumbPicture, look: GhostLook) {
        self.input(ThumbIn::DragStarted(id));
        let surface = self.surface.peek().clone();
        let (Some(surface), Some(spec)) = (surface, file_drag(picture, look)) else {
            eprintln!("sill: screenshot drag: no surface or no file URI; the card stays");
            self.input(ThumbIn::DragEnded(id, DragEnd::Refused));
            return;
        };
        Runtime::current().in_scope(self.scope, || {
            // A drag lasts seconds. If the compositor never reports its end, the card would stay
            // in `Dragging` with its hold stopped, on screen for good: this ends it as cancelled.
            let watchdog = spawn(async move {
                sleep(DRAG_LIMIT).await;
                eprintln!("sill: screenshot drag: no end after {DRAG_LIMIT:?}; the card stays");
                self.input(ThumbIn::DragEnded(id, DragEnd::Cancelled));
            });
            spawn(async move {
                let outcome = surface.start_drag(spec).await;
                watchdog.cancel();
                eprintln!("sill: screenshot drag: {outcome:?}");
                self.input(ThumbIn::DragEnded(id, drag_end(&outcome)));
            })
        });
    }

    fn perform(self, out: ThumbOut) {
        match out {
            ThumbOut::Show(_) => {
                self.link.peek().appeared(Transient::Thumbnail);
                self.visibility(Visibility::Shown);
            }
            ThumbOut::StartHold(token, ms) => {
                self.stop_hold();
                let wait = Duration::from_millis(u64::from(ms));
                let task = Runtime::current().in_scope(self.scope, || {
                    spawn(async move {
                        sleep(wait).await;
                        self.input(ThumbIn::HoldElapsed(token));
                    })
                });
                let mut hold = self.hold;
                hold.set(Some(task));
            }
            ThumbOut::Leave(id, why) => {
                self.stop_hold();
                self.link.peek().gone(Transient::Thumbnail);
                // A stopped service has nothing to act on: the thumbnail still goes.
                let _ = self.control.peek().send(ShotCommand::Leave(id, why));
            }
            ThumbOut::Hide => {
                self.stop_hold();
                self.link.peek().gone(Transient::Thumbnail);
                self.visibility(Visibility::Hidden);
            }
            ThumbOut::TakeKeyboard => self.take_keyboard(),
            ThumbOut::ReleaseKeyboard => self.keyboard(KeyboardMode::None),
        }
    }

    /// The keyboard on the picture, its focus ring drawn (the ask came from a shortcut, so the
    /// person is on the keyboard, though the seat has seen no key yet).
    fn take_keyboard(self) {
        self.keyboard(KeyboardMode::Exclusive);
        seat_on_keyboard(self.signals);
        Runtime::current().in_scope(self.scope, || {
            spawn(async move {
                let landed = focus_by_selector(PICTURE, Select::None).await;
                if let Err(error @ (FocusError::Busy | FocusError::Refused { .. })) = landed {
                    eprintln!("sill: screenshot thumbnail focus: {error}");
                }
            })
        });
    }

    /// Set the surface's keyboard mode; nothing without a host (a test, an SSR render).
    fn keyboard(self, mode: KeyboardMode) {
        if let Some(surface) = self.surface.peek().as_ref() {
            surface.set_keyboard(mode);
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
            surface.set_visibility(visibility);
        }
    }
}

/// The thumbnail's shared state, its timers owned by the calling component's scope.
pub(super) fn use_thumb(
    params: ThumbParams,
    control: Control<ShotCommand>,
    link: BannerLink,
) -> Thumb {
    let surface = try_use_context::<SurfaceHandle>();
    let signals = try_use_context::<HostSignals>();
    let thumb = use_hook(|| Thumb {
        state: Signal::new(ThumbState::Hidden),
        params: CopyValue::new(params),
        control: CopyValue::new(control),
        link: CopyValue::new(link),
        surface: CopyValue::new(surface),
        signals: CopyValue::new(signals),
        hold: CopyValue::new(None),
        scope: current_scope_id(),
    });
    if *thumb.params.peek() != params {
        let mut cell = thumb.params;
        cell.set(params);
    }
    thumb
}
