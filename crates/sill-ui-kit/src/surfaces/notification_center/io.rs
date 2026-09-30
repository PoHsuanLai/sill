//! The effects end of the center: follow the [`CenterLink`] through [`step_phase`] onto the
//! surface's visibility (quire's `Panel` plays the exit and says when it has settled), and carry
//! a row's [`RowAct`] to the notifications service. Once hidden nothing runs and nothing asks
//! for a frame.

use std::rc::Rc;

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::notifications::{
    Activation, ActivationToken, Clear, NotificationId, NotificationsCommand,
};
use sill_services::{Control, NotificationsService};

use super::groups::{Expanded, toggle_group};
use super::rows::RowAct;
use crate::hooks::use_control;
use crate::links::CenterLink;
use crate::panel::{Phase, PhaseInput, PhaseOut, Wanted, step_phase};

/// The column's phase on one output, and the surface it maps. Clone is cheap.
#[derive(Clone)]
pub(super) struct CenterPhase {
    phase: Signal<Phase>,
    surface: Option<SurfaceHandle>,
}

impl CenterPhase {
    /// Where the column is (read reactively).
    pub fn now(&self) -> Phase {
        (self.phase)()
    }

    /// The panel's exit has settled (its `on_hidden`): unmap.
    pub fn settled(&self) {
        drive(self.phase, self.surface.clone(), PhaseInput::ExitSettled);
    }
}

/// The column's phase on `output`, following the link.
pub(super) fn use_phase(output: OutputId) -> CenterPhase {
    let link = use_context::<CenterLink>();
    let shown = use_watch(link.subscribe());
    let surface = try_use_context::<SurfaceHandle>();
    let phase = use_signal(Phase::default);
    let follow = surface.clone();
    use_effect(move || {
        let wanted = match shown.read().open_on(output) {
            true => Wanted::Here,
            false => Wanted::NotHere,
        };
        drive(phase, follow.clone(), PhaseInput::Link(wanted));
    });
    CenterPhase { phase, surface }
}

/// `input` through the machine.
fn drive(mut phase: Signal<Phase>, surface: Option<SurfaceHandle>, input: PhaseInput) {
    let (next, out) = step_phase(*phase.peek(), input);
    if next != *phase.peek() {
        phase.set(next);
    }
    let visibility = match out {
        Some(PhaseOut::Map) => Visibility::Shown,
        Some(PhaseOut::Unmap) => Visibility::Hidden,
        // quire's `Panel` plays its exit on `shown: Hidden` and reports at its settle.
        Some(PhaseOut::PlayExit) | None => return,
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
    /// The column, which holds the keyboard.
    home: Signal<Option<Rc<MountedData>>>,
}

/// The handlers' context for one opening (its opened groups start closed).
pub(super) fn use_center_ctx() -> CenterCtx {
    CenterCtx {
        link: use_context::<CenterLink>(),
        control: use_control::<NotificationsService>(),
        surface: try_use_context::<SurfaceHandle>(),
        expanded: use_signal(Expanded::default),
        home: use_signal(|| None),
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
            RowAct::Dismiss(id) => {
                self.send(NotificationsCommand::Dismiss(id));
                self.take_keyboard();
            }
            RowAct::Clear(key) => {
                self.send(NotificationsCommand::ClearHistory(Clear::App(key)));
                self.take_keyboard();
            }
        }
    }

    /// The column mounted: keep it and give it the keyboard.
    pub fn home(&self, column: Rc<MountedData>) {
        let mut home = self.home;
        home.set(Some(column));
        self.take_keyboard();
    }

    /// Give the column the keyboard again: the button pressed (a Clear, Clear All, a card's
    /// close) leaves with its row, and with it the keyboard, so Escape reached nothing.
    fn take_keyboard(&self) {
        if let Some(column) = self.home.peek().clone() {
            ds::focus_soon(column);
        }
    }

    /// Clear All.
    pub fn clear_all(&self) {
        eprintln!("notification_center.act=ClearAll");
        self.send(NotificationsCommand::ClearHistory(Clear::All));
        self.take_keyboard();
    }

    /// Close the center (its exit plays first).
    pub fn close(&self) {
        self.link.close();
    }

    /// Invoke `action` with an activation token when the compositor gives one, then close.
    fn invoke(
        &self,
        id: NotificationId,
        action: sill_model::notifications::ActionKey,
        app: Option<String>,
    ) {
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
