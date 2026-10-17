//! The output volume: libpulse over pipewire-pulse, following the default sink.

mod backend;

use sill_model::volume::{Volume, VolumeCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The output volume as a [`Service`]: libpulse's standard mainloop on a thread of the blocking
/// pool, asleep until the sound server or a command wakes it, its callbacks feeding `step`. Without a pulse server the state stays at `Volume::default()`
/// (absent).
#[derive(Debug)]
pub enum VolumeService {}

impl Service for VolumeService {
    const NAME: &'static str = "volume";
    type State = Volume;
    type Command = VolumeCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let _ = env;
        async move {
            // The mainloop blocks: it runs on the blocking pool until `commands` closes, which
            // happens when every clone of the `Control` has dropped.
            let ServiceIo { state, commands } = io;
            backend::run(state, commands).await;
            Ok(())
        }
    }
}
