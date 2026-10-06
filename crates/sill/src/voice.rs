//! Hold-to-talk from the daemon's side: the three edges of the hold (`sill voice ...`), which
//! open the companion and start, finish or cancel the utterance, and the consent sheet's answer
//! written into `settings.toml`. The machines run in the voice service
//! (`sill_services::voice`); the settings keys are the ones `voiced` reads on every begin.

use ds_settings::Store;
use sill_ipc::response::Response;
use sill_ipc::show::VoiceRequest;
use sill_model::companion::CompanionCommand;
use sill_model::voice::{AskScope, VoiceCommand, VoiceConsent, VoiceTrigger};
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::{mpsc::UnboundedReceiver, watch};

use crate::companion::focus_of;
use crate::ipc::dispatch::{Reach, sent};

/// Answer `sill voice ...`. A hold begins by putting the launcher in companion mode (a summon no
/// app takes, so the prompt is the launcher's) and starting the utterance over what was focused.
pub fn dispatch(reach: &Reach, request: VoiceRequest) -> Response {
    let services = reach.services;
    let voice = |command| sent(services.voice.control.send(command), "voice");
    match request {
        VoiceRequest::HoldStart => {
            let focus = focus_of(services, &reach.links.launcher);
            let scope = AskScope {
                window: focus.window,
                app: focus.app.clone(),
            };
            let summon = sentless(services.companion.control.send(CompanionCommand::Summon(
                sill_model::companion::FocusInput { app: None, ..focus },
            )));
            match summon {
                Some(failed) => failed,
                None => voice(VoiceCommand::Start {
                    scope,
                    trigger: VoiceTrigger::HoldKey,
                }),
            }
        }
        VoiceRequest::HoldEnd => voice(VoiceCommand::Stop),
        VoiceRequest::HoldCancel => voice(VoiceCommand::Cancel),
    }
}

/// The failure response when the companion could not be told, else nothing.
fn sentless<E: std::fmt::Display>(result: Result<(), E>) -> Option<Response> {
    match sent(result, "companion") {
        Response::Done => None,
        failed => Some(failed),
    }
}

/// Write each answer of the consent sheet into `store`'s `settings.toml` (keeping every other
/// key) and publish the file on `settings`, so the service and `voiced` both see it.
pub fn spawn_consent_writer(
    rt: &Handle,
    mut consents: UnboundedReceiver<VoiceConsent>,
    store: Store,
    settings: watch::Sender<ShellFile>,
) {
    rt.spawn(async move {
        while let Some(consent) = consents.recv().await {
            if let Err(error) = write_consent(&store, &settings, consent) {
                eprintln!("sill: voice: {error}");
            }
        }
    });
}

/// Save `consent` as `voice.consent` and publish the file.
pub fn write_consent(
    store: &Store,
    settings: &watch::Sender<ShellFile>,
    consent: VoiceConsent,
) -> Result<(), String> {
    let mut file = crate::daemon::settings::load(store);
    file.voice.consent = consent;
    crate::daemon::settings::save(store, &file)
        .map_err(|error| format!("saving the settings: {error}"))?;
    settings.send_replace(file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use prov::UnixSeconds;
    use sill_model::voice::VoiceConsent;
    use sill_settings::ShellFile;
    use tokio::sync::watch;

    use super::write_consent;

    fn scratch() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sill-voice-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The file `voiced` parses: its `[voice]` table holds exactly the forms it reads.
    #[test]
    fn an_answer_writes_the_keys_voiced_reads_and_keeps_the_others() {
        let dir = scratch();
        let store = crate::daemon::settings::store(&dir);
        let mut before = ShellFile::default();
        before.companion.notify = sill_settings::NotifyWhen::Never;
        crate::daemon::settings::save(&store, &before).unwrap_or_else(|e| panic!("{e}"));
        let (tx, rx) = watch::channel(before);
        let table_of = || -> toml::Table {
            std::fs::read_to_string(dir.join("sill/settings.toml"))
                .unwrap_or_else(|e| panic!("{e}"))
                .parse()
                .unwrap_or_else(|e| panic!("{e}"))
        };
        write_consent(&store, &tx, VoiceConsent::Given(UnixSeconds(1_760_000_000)))
            .unwrap_or_else(|e| panic!("{e}"));
        let table = table_of();
        assert_eq!(
            table["voice"]["consent"]["given"].as_integer(),
            Some(1_760_000_000)
        );
        assert_eq!(table["companion"]["notify"].as_str(), Some("never"));
        assert_eq!(
            rx.borrow().voice.consent,
            VoiceConsent::Given(UnixSeconds(1_760_000_000))
        );
        write_consent(&store, &tx, VoiceConsent::Declined).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(table_of()["voice"]["consent"].as_str(), Some("declined"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
