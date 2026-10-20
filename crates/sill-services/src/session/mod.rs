//! The session: logind's power actions and what it currently allows. Every surface that offers
//! them reads this service's state and sends to its control: the bar's system menu, the power
//! menu (design/20 §1.8) and the launcher's System rows. Only this service asks logind.

mod backend;

use std::sync::Arc;

use sill_model::locker::LockerCommand;
use sill_model::session::{Session, SessionCommand};

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// logind's power actions as a [`Service`]. Without logind the state stays at
/// `Session::default()` (absent: nothing offered), and the power menu shows every button
/// unavailable. Its `Deps` is the locker's control, where `Run(Lock)` goes; the other actions go
/// to `env`'s session actions.
#[derive(Debug)]
pub enum SessionService {}

impl Service for SessionService {
    const NAME: &'static str = "session";
    type State = Session;
    type Command = SessionCommand;
    type Deps = Control<LockerCommand>;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.system.clone();
        let actions = Arc::clone(&env.session_actions);
        let hint = env.session_id.clone();
        async move {
            backend::run(bus, actions, hint, io.state, io.commands, deps).await;
            Ok(())
        }
    }
}
