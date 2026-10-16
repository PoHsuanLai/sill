//! sill serves the screen-sharing picker: `org.quire.Shell1.PickCaptureSource`, which transom (the
//! portal backend) calls with what an app asked to share and which returns what the person picked;
//! and the questions beside it (`PickColor`, `AskPermission`).
//!
//! - **The name.** The object is exported on the connection the intents service already holds as
//!   `org.quire.Shell` (the seat); this service claims no name of its own. Without the seat the
//!   interface is not served and transom's call fails with an unknown-name error, which it takes
//!   as "nobody to ask".
//! - **Only transom.** A call is let in only from the program the table of the desktop's own
//!   programs names as the portal backend, which owns the portal backend's name
//!   ([`crate::callers`]); any other caller is refused and draws nothing.
//! - **One at a time.** A sharing or colour call while a sheet is open gets a plain error at
//!   once. A permission question waits its turn instead (below).
//! - **Candidates.** Screens come from the displays service; windows are pushed by the daemon
//!   from the compositor arm's `DesktopView` ([`CaptureCommand::Windows`]), by the identifiers
//!   casement publishes. A call waits on the person (Allow or Cancel on the sheet) and returns
//!   an empty list on cancel. The caller leaving the bus takes the sheet down.
//! - **Colours.** `PickColor` is checked and admitted the same way, one question of either
//!   kind at a time, and answered with `a(ddd)`: one colour in 0..1, or an empty array when the
//!   person cancelled. The daemon sends [`CaptureCommand::Eyedropper`] while an eyedropper
//!   surface exists (`sill_overlays::eyedropper`); without one it is answered with an empty array
//!   at once.
//! - **Permission.** `AskPermission` is the portal's yes-or-no question about an app (transom asks
//!   it before remote control and before a screenshot the person did not start): the texts are
//!   cleaned and cut (`PermissionAsk::from_wire`), checked like the others, and answered with 0
//!   (the yes button), 1 (the no button) or 2 (the sheet went away unanswered). Its sheet is
//!   `sill_overlays::permission_sheet`.
//! - **The compositor's questions.** When a program wants a power (see the screen, type for the
//!   person, read the clipboard) the compositor asks the shell on its own protocol
//!   ([`CaptureCommand::Consent`]); the same sheet asks the person, in the same line as
//!   `AskPermission`: one sheet at a time, the rest waiting in the order they came. The answer
//!   goes back as a [`DesktopCommand`] on [`CaptureLinks::answers`].
//! - **Passive until [`CaptureCommand::Serve`]**: a test's `start` exports nothing.

mod backend;

use std::sync::Arc;

use sill_model::apps::AppIndex;
use sill_model::capture_pick::{CaptureCommand, CapturePicker};
use sill_model::desktop::DesktopCommand;
use sill_model::displays::Displays;
use tokio::sync::{mpsc, watch};

use crate::env::Env;
use crate::error::ServiceError;
use crate::seat::SeatView;
use crate::service::{Service, ServiceIo};

/// What the service reads: the seat to export on, the displays to list as screens, the installed
/// apps that give a program its name, and where the compositor's answers go.
#[derive(Debug)]
pub struct CaptureLinks {
    /// The shell's connection at intentd, which owns `org.quire.Shell`.
    pub seat: SeatView,
    pub displays: watch::Receiver<Displays>,
    /// The installed apps, which name the program that a compositor's question is about.
    pub apps: watch::Receiver<AppIndex>,
    /// Where an answer to the compositor's question goes
    /// ([`DesktopCommand::AnswerPermission`]), for the daemon to carry to the compositor.
    pub answers: mpsc::UnboundedSender<DesktopCommand>,
}

/// The picker as a [`Service`].
#[derive(Debug)]
pub enum CaptureService {}

impl Service for CaptureService {
    const NAME: &'static str = "capture-pick";
    type State = CapturePicker;
    type Command = CaptureCommand;
    type Deps = CaptureLinks;

    fn run(
        env: &Env,
        deps: CaptureLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let callers = Arc::clone(&env.callers);
        async move {
            backend::run(deps, io, callers).await;
            Ok(())
        }
    }
}
