//! inferd (`org.quire.Inference1`), as the shell hears it: when another of the person's computers
//! asks to borrow this computer's models, inferd says so and the shell asks "Let <name> use this
//! computer's models?" in a notification with Allow and Deny. A press is given to inferd as
//! `AnswerGuest`. The notification is taken back when the question is answered, here or in
//! Settings.
//!
//! The service subscribes to `GuestAsks` and `GuestsChanged`, then reads `Guests` once for the
//! questions already waiting, and again whenever `org.quire.Inference1` gets a new owner. Without
//! inferd the state stays empty.

mod backend;

use sill_model::guest_asks::{GuestAsksCommand, GuestAsksState};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::GuestAsksLinks;

/// The questions about lending models on `env`'s session bus, told through the notifications
/// service in its [`GuestAsksLinks`].
#[derive(Debug)]
pub enum GuestAsksService {}

impl Service for GuestAsksService {
    const NAME: &'static str = "guest_asks";
    type State = GuestAsksState;
    type Command = GuestAsksCommand;
    type Deps = GuestAsksLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move { backend::run(bus, deps, io).await }
    }
}
