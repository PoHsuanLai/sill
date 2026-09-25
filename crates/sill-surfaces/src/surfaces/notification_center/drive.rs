//! The effects end of the center: follow the [`CenterLink`] through [`step_phase`] onto the
//! surface's visibility, time the column's exit on quire's clock (`use_motion_timer`), and carry
//! a row's [`RowAct`] to the notifications service. Once hidden nothing runs and nothing asks
//! for a frame.

use dioxus::prelude::*;
use ds::{Anim, MotionTimer, use_motion_timer};
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_services::{
    Activation, ActivationToken, Clear, Control, NotificationId, NotificationsCommand,
};

use super::groups::{Expanded, toggle_group};
use super::link::CenterLink;
use super::phase::{Phase, PhaseInput, PhaseOut, Wanted, step_phase};
use super::rows::RowAct;
use crate::hooks::use_services;

/// The column's exit: quire's `pane-out-r`, the mirror of the entrance (FINDINGS Q123).
pub(super) const EXIT: Anim = Anim::PaneOutR;

/// The column's entrance: quire's `slide-r` at `--t-move --e-spring`, in from the right.
pub(super) const ENTRANCE: Anim = Anim::PaneInR;

/// The column's phase on `output`, following the link.
pub(super) fn use_phase(output: OutputId) -> Signal<Phase> {
    let link = use_context::<CenterLink>();
    let shown = use_watch(link.subscribe());
    let surface = try_use_context::<SurfaceHandle>();
    let phase = use_signal(Phase::default);
    let exit = use_motion_timer(EXIT);
    use_effect(move || {
        let wanted = match shown.read().open_on(output) {
            true => Wanted::Here,
            false => Wanted::NotHere,
        };
        drive(phase, surface.clone(), exit, PhaseInput::Link(wanted));
    });
    phase
}

/// `input` through the machine; the exit's settle comes back here.
fn drive(
    mut phase: Signal<Phase>,
    surface: Option<SurfaceHandle>,
    exit: MotionTimer,
    input: PhaseInput,
) {
    let (next, out) = step_phase(*phase.peek(), input);
    if next != *phase.peek() {
        phase.set(next);
    }
    let visibility = match out {
        Some(PhaseOut::Map) => Visibility::Shown,
        Some(PhaseOut::Unmap) => Visibility::Hidden,
        Some(PhaseOut::PlayExit) => {
            let settle = surface.clone();
            exit.start(EventHandler::new(move |()| {
                drive(phase, settle.clone(), exit, PhaseInput::ExitSettled);
            }));
            return;
        }
        None => return,
    };
    eprintln!(
        "notification_center.visible={}",
        visibility == Visibility::Shown
    );
    if let Some(surface) = &surface {
        surface.apply(SurfaceOp::SetVisibility(visibility));
    }
}

/// What the column's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct CenterCtx {
    pub link: CenterLink,
    control: Control<NotificationsCommand>,
    surface: Option<SurfaceHandle>,
    pub expanded: Signal<Expanded>,
}

/// The handlers' context for one opening (its opened groups start closed).
pub(super) fn use_center_ctx() -> CenterCtx {
    CenterCtx {
        link: use_context::<CenterLink>(),
        control: use_services().notifications_control,
        surface: try_use_context::<SurfaceHandle>(),
        expanded: use_signal(Expanded::default),
    }
}

impl CenterCtx {
    /// Carry out `act`; `app` is the sender's desktop entry, for the activation token.
    pub fn act(&self, act: RowAct, app: Option<String>) {
        eprintln!("notification_center.act={act:?}");
        match act {
            RowAct::Invoke(id, action) => self.invoke(id, action, app),
            RowAct::Toggle(key) => {
                let mut expanded = self.expanded;
                let next = toggle_group(expanded.peek().clone(), &key);
                expanded.set(next);
            }
            RowAct::Dismiss(id) => self.send(NotificationsCommand::Dismiss(id)),
            RowAct::Clear(key) => self.send(NotificationsCommand::ClearHistory(Clear::App(key))),
        }
    }

    /// Clear All.
    pub fn clear_all(&self) {
        eprintln!("notification_center.act=ClearAll");
        self.send(NotificationsCommand::ClearHistory(Clear::All));
    }

    /// Close the center (its exit plays first).
    pub fn close(&self) {
        self.link.close();
    }

    /// Invoke `action` with an activation token when the compositor gives one, then close.
    fn invoke(&self, id: NotificationId, action: sill_services::ActionKey, app: Option<String>) {
        let (surface, control, link) = (
            self.surface.clone(),
            self.control.clone(),
            self.link.clone(),
        );
        spawn(async move {
            let token = match &surface {
                Some(surface) => surface.activation_token(app).await,
                None => None,
            };
            let token = token.map_or(Activation::NoToken, |t| {
                Activation::Token(ActivationToken(t))
            });
            if let Err(error) =
                control.send(NotificationsCommand::InvokeAction { id, action, token })
            {
                eprintln!("sill: notification center: {error}");
            }
            link.close();
        });
    }

    fn send(&self, command: NotificationsCommand) {
        if let Err(error) = self.control.send(command) {
            eprintln!("sill: notification center: {error}");
        }
    }
}
