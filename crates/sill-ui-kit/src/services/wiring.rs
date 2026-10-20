//! What `Services::start` is given besides the seams in `Env`: the channels the daemon reads
//! what the services send out of, and the ones several services share.

use std::fmt;

use sill_model::companion::SummonOut;
use sill_model::desktop::{DesktopCommand, DesktopView};
use sill_model::voice::VoiceConsent;
use sill_services::intents::ShellCall;
use sill_services::notifications::Answers;
use sill_services::seat::{SeatPublisher, SeatView, seat};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;

/// The ends the daemon holds, and the channels between services that no service owns.
pub struct Wiring {
    /// Where the shell's own actions go.
    pub shell_calls: UnboundedSender<ShellCall>,
    /// Where a summon's launcher effects go.
    pub summons: UnboundedSender<SummonOut>,
    /// Where the answer to the voice consent sheet goes.
    pub consents: UnboundedSender<VoiceConsent>,
    /// Where the answer to a question the compositor asked goes.
    pub permission_answers: UnboundedSender<DesktopCommand>,
    /// The connection the intents service claims `org.quire.Shell` on, which the companion
    /// speaks on as the launcher.
    pub seat_publisher: SeatPublisher,
    pub seat_view: SeatView,
    /// The windows service writes the desktop's view; every reader of it holds the receiver.
    pub desktop_tx: watch::Sender<DesktopView>,
    pub desktop: watch::Receiver<DesktopView>,
    /// What the person does to a notification, which the services with notices hear.
    pub answers: Answers,
}

impl Wiring {
    /// The daemon's ends, with a seat nobody holds yet and an empty desktop.
    pub fn new(
        shell_calls: UnboundedSender<ShellCall>,
        summons: UnboundedSender<SummonOut>,
        consents: UnboundedSender<VoiceConsent>,
        permission_answers: UnboundedSender<DesktopCommand>,
    ) -> Wiring {
        let (seat_publisher, seat_view) = seat();
        let (desktop_tx, desktop) = watch::channel(DesktopView::default());
        Wiring {
            shell_calls,
            summons,
            consents,
            permission_answers,
            seat_publisher,
            seat_view,
            desktop_tx,
            desktop,
            answers: Answers::default(),
        }
    }
}

impl fmt::Debug for Wiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wiring").finish_non_exhaustive()
    }
}
