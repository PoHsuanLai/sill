//! sill's seat at the companion: the double tap and the summon that decide where the prompt
//! opens, the questions the launcher asks (`org.quire.Companion1`), the answers that come back,
//! the things apps index (`org.quire.Intents1.Search`), the undo journal, the computer-use runs
//! (`org.quire.Cua1`) and the memory timeline (`org.quire.Memory1`).
//!
//! Everything said to intentd and the companion goes out on the connection the intents service
//! holds as `org.quire.Shell` (`seat`): intentd names a caller's role from the names its
//! connection owns, so a turn or a search from any other connection would not be the launcher's.
//! Without the seat the machines still run (a summon opens the launcher) and every lookup is
//! `Failed`.
//!
//! The machines' effects that the daemon carries out (open, switch or close the launcher) leave
//! on [`CompanionLinks::summons`]; the service asks the app itself.

mod backend;

use tokio::sync::mpsc;

use sill_model::companion::{Companion, CompanionCommand, SummonOut};

use crate::env::Env;
use crate::error::ServiceError;
use crate::seat::SeatView;
use crate::service::{Service, ServiceIo};

/// What the service needs from the daemon: the seat to speak on, and where a summon's launcher
/// effects go.
#[derive(Debug)]
pub struct CompanionLinks {
    /// The shell's connection at intentd.
    pub seat: SeatView,
    /// The launcher effects of a summon (`OpenLauncher`, `SwitchLauncher`, `CloseLauncher`).
    pub summons: mpsc::UnboundedSender<SummonOut>,
}

/// The companion as a [`Service`].
#[derive(Debug)]
pub enum CompanionService {}

impl Service for CompanionService {
    const NAME: &'static str = "companion";
    type State = Companion;
    type Command = CompanionCommand;
    type Deps = CompanionLinks;

    fn run(
        env: &Env,
        deps: CompanionLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move {
            backend::run(reach, deps, io).await;
            Ok(())
        }
    }
}
