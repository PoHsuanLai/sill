//! The consent sheet's answer, end to end on a private bus and a scratch config directory: a
//! stand-in `voiced` that decides from the `[voice]` table of the very file sill writes, the voice
//! service refused with `NeedsConsent`, the answer turned into the consent the daemon writes, and
//! the next hold either begun (Allow) or never attempted (Not Now). Never the person's `~/.config`.

use crate::voice_support::{BeginAnswer, Desk, desk_or_return};
use ds_settings::{ConfigRoot, Store};
use sill::policy::Policy;
use sill::policy::voice::VoiceBridge;
use sill::voice::write_consent;
use sill_launcher::WindowKey;
use sill_model::voice::{
    AskScope, ConsentAnswer, ConsentSheet, Notice, Phase, VoiceCommand, VoiceParams,
};
use sill_settings::{APP, ShellFile};
use sill_testkit::TempDir;
use tokio::sync::watch;
use voice_wire::{VoiceRefusal, VoiceTrigger};

fn start() -> VoiceCommand {
    VoiceCommand::Start {
        scope: AskScope {
            window: Some(WindowKey(4)),
            app: None,
        },
        trigger: VoiceTrigger::HoldKey,
    }
}

fn begins(desk: &Desk) -> usize {
    desk.script
        .lines()
        .iter()
        .filter(|line| line.starts_with("begin "))
        .count()
}

/// Ask, answer, write the consent the service sent, and tell the service what the file says now,
/// as the daemon's policy would. Returns the settings text.
async fn answered(desk: &mut Desk, answer: ConsentAnswer, dir: &TempDir) -> toml::Table {
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.sheet, ConsentSheet::Asking).then_some(()))
        .await;
    desk.send(VoiceCommand::Answer(answer));
    let consent = tokio::time::timeout(std::time::Duration::from_secs(60), desk.consents.recv())
        .await
        .unwrap_or_else(|_| panic!("no consent was sent"))
        .unwrap_or_else(|| panic!("closed"));
    let store = Store::new(ConfigRoot::Scratch(dir.path().to_path_buf()), APP);
    let (tx, rx) = watch::channel(ShellFile::default());
    write_consent(&store, &tx, consent).unwrap_or_else(|e| panic!("{e}"));
    desk.send(VoiceCommand::Params(VoiceBridge.of(&rx.borrow())));
    desk.state(|v| (v.params.consent == consent).then_some(()))
        .await;
    std::fs::read_to_string(dir.path().join("sill/settings.toml"))
        .unwrap_or_else(|e| panic!("{e}"))
        .parse()
        .unwrap_or_else(|e| panic!("{e}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn allow_writes_the_given_consent_and_the_next_hold_begins() {
    let dir = TempDir::new("voice-consent-allow");
    let file = dir.path().join("sill/settings.toml");
    let mut desk = desk_or_return!(BeginAnswer::FromSettings(file));
    assert_eq!(begins(&desk), 0);
    let table = answered(&mut desk, ConsentAnswer::Allow, &dir).await;
    assert_eq!(begins(&desk), 1, "the first hold was refused");
    assert!(
        table["voice"]["consent"]["given"]
            .as_integer()
            .is_some_and(|at| at > 0),
        "{table:?}"
    );
    assert_eq!(
        table["voice"]["hold_to_talk"].as_str(),
        Some("hold_command")
    );
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    assert_eq!(
        begins(&desk),
        2,
        "stand-in voiced read the file and said yes"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn not_now_writes_declined_and_the_next_hold_never_calls_begin() {
    let dir = TempDir::new("voice-consent-not-now");
    let file = dir.path().join("sill/settings.toml");
    let mut desk = desk_or_return!(BeginAnswer::FromSettings(file));
    let table = answered(&mut desk, ConsentAnswer::NotNow, &dir).await;
    assert_eq!(table["voice"]["consent"].as_str(), Some("declined"));
    assert_eq!(begins(&desk), 1);
    desk.send(start());
    // Told again that nothing was decided, the next hold does begin and is refused by the file:
    // that is the second call the stand-in sees, so the declined hold made none.
    desk.send(VoiceCommand::Params(VoiceParams::default()));
    desk.send(start());
    desk.state(|v| matches!(v.notice, Notice::Refused(VoiceRefusal::Disabled)).then_some(()))
        .await;
    assert_eq!(
        begins(&desk),
        2,
        "sill itself did not begin while voice was declined"
    );
    assert_eq!(desk.voice.state.borrow().sheet, ConsentSheet::Closed);
}
