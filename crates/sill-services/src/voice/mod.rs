//! sill's seat at voiced: hold-to-talk (`org.quire.Voice1`). A hold begins an utterance, the
//! words come back on a stream, and what was said goes to the companion as the question
//! (`CompanionCommand::Ask`, spoken). A refusal is a state the surfaces draw: the consent sheet
//! for `NeedsConsent`, a plain line for the rest.
//!
//! Everything said to voiced goes out on the connection the intents service holds as
//! `org.quire.Shell` (`seat`): voiced names the caller's role from the names that connection
//! owns, so a begin from any other connection would be refused. Without the seat nothing can
//! begin and a hold says so.

mod backend;

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::seat::SeatView;
use crate::service::{Service, ServiceIo};
use sill_model::companion::CompanionCommand;
use sill_model::voice::{Voice, VoiceCommand, VoiceConsent};
use tokio::sync::mpsc;

/// What the service needs from the daemon: the seat to speak on, and the companion that takes
/// the question.
#[derive(Debug, Clone)]
pub struct VoiceLinks {
    /// The shell's connection at intentd.
    pub seat: SeatView,
    /// Where the transcript goes as a spoken ask.
    pub companion: Control<CompanionCommand>,
    /// Where the answer to the consent sheet goes, as the consent to write into the settings.
    pub consents: mpsc::UnboundedSender<VoiceConsent>,
}

/// Hold-to-talk as a [`Service`].
#[derive(Debug)]
pub enum VoiceService {}

impl Service for VoiceService {
    const NAME: &'static str = "voice";
    type State = Voice;
    type Command = VoiceCommand;
    type Deps = VoiceLinks;

    fn run(
        env: &Env,
        deps: VoiceLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move {
            backend::run(reach, deps, io).await;
            Ok(())
        }
    }
}
