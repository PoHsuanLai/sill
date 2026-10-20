//! Next Input Source (the Mac's Control-Space) on the session bus. The running input-method
//! framework is looked for when the action fires, never assumed: fcitx5's
//! `org.fcitx.Fcitx.Controller1` is switched to the entry after the current one in its current
//! group ([`sill_model::input_source::next_of`]). IBus keeps its own bus (found through an address
//! file), which is more than a small proxy, so it is a typed refusal; so is no framework at all.
//! The outcome is the service's state.

mod backend;

use sill_model::input_source::{InputCommand, InputSwitch};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Next Input Source as a [`Service`]. The state is `InputSwitch::Idle` until the first switch.
#[derive(Debug)]
pub enum InputSourceService {}

impl Service for InputSourceService {
    const NAME: &'static str = "input_source";
    type State = InputSwitch;
    type Command = InputCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.session.clone(), io.state, io.commands)
    }
}
