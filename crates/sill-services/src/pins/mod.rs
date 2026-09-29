//! The dock's pinned apps, `~/.config/sill/dock.json` (design/10 §10.6: "pinned items stay in
//! `~/.config/sill/dock.json`"; state, not a setting, design/22 §1 rule 3).

mod backend;

use sill_model::pins::{DockPins, PinEvent};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The dock pins as a [`Service`]: loaded at once, then every [`PinEvent`] sent to the control
/// applied and the file written atomically after each; an external edit (another process, a
/// text editor) is picked up through a 30 ms debounced directory watch and published as if it
/// were a `Replaced` event.
#[derive(Debug)]
pub enum PinsService {}

impl Service for PinsService {
    const NAME: &'static str = "pins";
    type State = DockPins;
    type Command = PinEvent;
    type Deps = ();

    fn initial(env: &Env, (): &()) -> DockPins {
        backend::load(&env.dirs.sill_config())
    }

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let dir = env.dirs.sill_config();
        async move {
            backend::run(dir, io.state, io.commands).await;
            Ok(())
        }
    }
}
