//! The Quire arm over any [`Desktop`]: the wire client on the shell host's connection, or the
//! fake in tests. It turns what the compositor said into the shell's view and notes, and the
//! shell's commands into requests.

use std::collections::BTreeMap;

use casement_protocols::vocab::ShellRole;
use desktop_client::capture::CaptureFailure;
use desktop_client::quire::QuireEvent;
use desktop_client::{
    CaptureId, Desktop, DesktopCommand as Wire, DesktopEvent, Frame, SurfaceKey, Ticket,
};
use porter_core::SpaceId;
use sill_model::desktop::{DesktopCommand, DesktopView, ShellNote, SpacePicture};

use super::command::quire_wire_of;
use super::notes::note_of;
use super::pictures::picture_of;
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
    /// The pictures of Spaces asked for and not yet taken, by capture, with the output and Space
    /// each is of.
    pending: BTreeMap<CaptureId, (String, SpaceId)>,
}

impl<D: Desktop, S: NoteSink> QuireArm<D, S> {
    /// An arm over `desktop`, its notes to `sink`.
    pub fn new(desktop: D, sink: S) -> Self {
        Self {
            desktop,
            sink,
            surfaces: ShellSurfaces::default(),
            pending: BTreeMap::new(),
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
        for request in requests {
            let ticket = self.desktop.send(request).map_err(ArmError::Send)?;
            if let (Ticket::Capture(id), DesktopCommand::CaptureSpace { output, space, .. }) =
                (ticket, command)
            {
                self.pending.insert(id, (output.clone(), space.clone()));
            }
        }
        Ok(())
    }

    /// Hand the sink a [`ShellNote::SpacePicture`] for each asked-for picture that is finished.
    /// `take` takes a finished capture out of the desktop (the wire client has a method for it,
    /// the trait has none) and says nothing while it is still going.
    pub fn pictures(
        &mut self,
        mut take: impl FnMut(&mut D, CaptureId) -> Option<Result<Frame, CaptureFailure>>,
    ) {
        let ids: Vec<CaptureId> = self.pending.keys().copied().collect();
        for id in ids {
            let Some(done) = take(&mut self.desktop, id) else {
                continue;
            };
            let Some((output, space)) = self.pending.remove(&id) else {
                continue;
            };
            let picture = done.map_or(SpacePicture::Failed, |frame| picture_of(&frame));
            self.sink.note(ShellNote::SpacePicture {
                output,
                space,
                picture,
            });
        }
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
