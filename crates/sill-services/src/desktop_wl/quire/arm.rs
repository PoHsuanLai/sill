//! The Quire arm over any [`Desktop`]: the wire client on the shell host's connection, or the
//! fake in tests. It turns what the compositor said into the shell's view and notes, and the
//! shell's commands into requests.

use casement_protocols::vocab::ShellRole;
use desktop_client::quire::QuireEvent;
use desktop_client::{Desktop, DesktopCommand as Wire, DesktopEvent, SurfaceKey};
use sill_model::desktop::{DesktopCommand, DesktopView, ShellNote};

use super::command::quire_wire_of;
use super::notes::note_of;
use super::surfaces::ShellSurfaces;
use super::view::quire_view;
use crate::desktop_wl::Refusal;

/// Where the shell protocol's notes go: whatever draws or acts on them.
pub trait NoteSink {
    /// The compositor said `note`.
    fn note(&mut self, note: ShellNote);
}

/// A sink that keeps every note, oldest first.
impl NoteSink for Vec<ShellNote> {
    fn note(&mut self, note: ShellNote) {
        self.push(note);
    }
}

/// Why a command did not go out.
#[derive(Debug, thiserror::Error)]
pub enum ArmError<E: std::error::Error + 'static> {
    /// The command names something the desktop does not have, or the arm cannot say.
    #[error("{0}")]
    Refused(#[from] Refusal),
    /// The client refused to send it.
    #[error("{0}")]
    Send(#[source] E),
}

/// The Quire arm: a [`Desktop`], where its notes go and the surfaces registered with it.
#[derive(Debug)]
pub struct QuireArm<D, S> {
    desktop: D,
    sink: S,
    surfaces: ShellSurfaces,
}

impl<D: Desktop, S: NoteSink> QuireArm<D, S> {
    /// An arm over `desktop`, its notes to `sink`.
    pub fn new(desktop: D, sink: S) -> Self {
        Self {
            desktop,
            sink,
            surfaces: ShellSurfaces::default(),
        }
    }

    /// The desktop, for what its own type offers (the wire client registers surfaces).
    pub fn desktop_mut(&mut self) -> &mut D {
        &mut self.desktop
    }

    /// The sink the notes go to.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Hand the notes the compositor sent since the last call to the sink, in order, and return
    /// the desktop as the shell sees it.
    pub fn sync(&mut self) -> DesktopView {
        for event in self.desktop.drain_events() {
            if let DesktopEvent::Quire(QuireEvent::Shell(shell)) = &event
                && let Some(note) = note_of(shell)
            {
                self.sink.note(note);
            }
        }
        quire_view(self.desktop.state())
    }

    /// Carry out `command`.
    pub fn execute(&mut self, command: &DesktopCommand) -> Result<(), Failed<D>> {
        let requests = quire_wire_of(command, self.desktop.state(), &self.surfaces)?;
        requests
            .into_iter()
            .try_for_each(|request| self.desktop.send(request).map(drop))
            .map_err(ArmError::Send)
    }

    /// Tell the compositor that `surface` (already registered with the client as `key`) is the
    /// shell's `role` on `output`, and remember it for the commands that name a surface.
    pub fn assign(
        &mut self,
        role: ShellRole,
        output: impl Into<String>,
        key: SurfaceKey,
    ) -> Result<(), Failed<D>> {
        self.desktop
            .send(Wire::SetShellRole { surface: key, role })
            .map_err(ArmError::Send)?;
        self.surfaces.add(role, output, key);
        Ok(())
    }
}

/// What a command of an arm over `D` fails with.
type Failed<D> = ArmError<<D as Desktop>::Error>;
