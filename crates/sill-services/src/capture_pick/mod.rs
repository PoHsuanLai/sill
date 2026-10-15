//! sill serves the screen-sharing picker: `org.quire.Shell1.PickCaptureSource`, which transom (the
//! portal backend) calls with what an app asked to share and which returns what the person picked.
//!
//! - **The name.** The object is exported on the connection the intents service already holds as
//!   `org.quire.Shell` (the seat); this service claims no name of its own. Without the seat the
//!   interface is not served and transom's call fails with an unknown-name error, which it takes
//!   as "nobody to ask".
//! - **Only transom.** A call is checked against the owner of the portal backend's name; any
//!   other caller is refused and draws nothing.
//! - **One at a time.** A call while a sheet is open gets a plain error at once.
//! - **Candidates.** Screens come from the displays service; windows are pushed by the daemon
//!   from the compositor arm's `DesktopView` ([`CaptureCommand::Windows`]), by the identifiers
//!   casement publishes. A call waits on the person (Allow or Cancel on the sheet) and returns
//!   an empty list on cancel. The caller leaving the bus takes the sheet down.
//! - **Colours.** `PickColor` is checked and admitted the same way, one question of either
//!   kind at a time, and answered with `a(ddd)`: one colour in 0..1, or an empty array when the
//!   person cancelled. While no surface draws the eyedropper
//!   ([`CaptureCommand::Eyedropper`] was not sent) it is answered with an empty array at once.
//! - **Passive until [`CaptureCommand::Serve`]**: a test's `start` exports nothing.

mod backend;

use sill_model::capture_pick::{CaptureCommand, CapturePicker};
use sill_model::displays::Displays;
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::seat::SeatView;
use crate::service::{Service, ServiceIo};

/// What the service reads: the seat to export on and the displays to list as screens.
#[derive(Debug)]
pub struct CaptureLinks {
    /// The shell's connection at intentd, which owns `org.quire.Shell`.
    pub seat: SeatView,
    pub displays: watch::Receiver<Displays>,
}

/// The picker as a [`Service`].
#[derive(Debug)]
pub enum CaptureService {}

impl Service for CaptureService {
    const NAME: &'static str = "capture-pick";
    type State = CapturePicker;
    type Command = CaptureCommand;
    type Deps = CaptureLinks;

    #[expect(
        clippy::manual_async_fn,
        reason = "an async fn would capture the borrowed env; the trait wants an owned future"
    )]
    fn run(
        _env: &Env,
        deps: CaptureLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        async move {
            backend::run(deps, io).await;
            Ok(())
        }
    }
}
