//! The output volume: libpulse over pipewire-pulse on a threaded mainloop, following the
//! default sink.

mod live;
mod model;

pub use model::{Mute, Step, Volume, VolumeCommand, VolumeEvent, step, stepped};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The output volume as a [`Service`]: libpulse's threaded mainloop on its own OS thread, its
/// callbacks feeding [`step`]. Without a pulse server the state stays at `Volume::default()`
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
            // libpulse's threaded mainloop blocks: it runs on the blocking pool until `commands`
            // closes, which happens when every clone of the `Control` has dropped.
            let ServiceIo { state, commands } = io;
            let _ = tokio::task::spawn_blocking(move || live::run(state, commands)).await;
            Ok(())
        }
    }
}
