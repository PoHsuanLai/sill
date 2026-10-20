//! Where the notifications server says what the person did to a notification, for the services
//! that posted notices of their own.

use sill_model::notifications::Said;
use tokio::sync::broadcast;

/// How many answers wait for a slow listener before it misses the oldest.
const HELD: usize = 64;

/// The answers of the person to notifications: an action invoked, a notification closed. An event
/// stream, not state: a listener hears what is said after it listens. Cloning shares the channel.
#[derive(Debug, Clone)]
pub struct Answers(broadcast::Sender<Said>);

impl Default for Answers {
    fn default() -> Answers {
        Answers(broadcast::channel(HELD).0)
    }
}

impl Answers {
    /// A receiver of everything said from now on. Taken once per run of a listening service.
    pub fn hear(&self) -> broadcast::Receiver<Said> {
        self.0.subscribe()
    }

    /// Say `said` to every listener.
    pub fn say(&self, said: Said) {
        // Nobody listening has nobody to tell.
        let _ = self.0.send(said);
    }
}
