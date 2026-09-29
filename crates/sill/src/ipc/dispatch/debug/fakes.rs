//! The made-up inputs: each is the value the service would have received from the real source,
//! sent to the service directly.

use std::path::PathBuf;

use sill_ipc::debug::{CornerName, OsdTopic};
use sill_ipc::response::Response;
use sill_launcher::AppId;
use sill_model::auth::FakeVerdict;
use sill_model::notifications::{NotificationsCommand, Posting};
use sill_model::polkit_agent::{FakeRequest, PolkitCommand};
use sill_model::screenshots::ShotCommand;
use sill_model::units::Percent;
use sill_model::volume::Mute;
use sill_surfaces::links::SwitcherAsk;

use super::super::reach::Reach;
use super::super::reply::{sent, unsupported};
use super::super::switcher;

/// `sill debug osd`: the OSD for a change that moves no real level (the session's default sink
/// may be somebody's headphones).
pub fn osd(reach: &Reach, topic: OsdTopic, percent: Percent, mute: Mute) -> Response {
    let (kind, level) = crate::osd::fake_of(topic, percent, mute);
    crate::osd::fake(
        &reach.links.osd,
        reach.services,
        reach.outputs,
        &reach.settings.borrow(),
        kind,
        level,
    );
    Response::Done
}

/// `sill debug notify`: through the same reducer as a `Notify`, so it shows a banner and enters
/// the history.
pub fn notify(reach: &Reach, posting: Posting) -> Response {
    sent(
        reach
            .services
            .notifications
            .control
            .send(NotificationsCommand::Post(Box::new(posting))),
        "notifications",
    )
}

/// `sill debug screenshot`: the thumbnail for a file, as if the folder watch had seen it.
pub fn screenshot(reach: &Reach, path: PathBuf) -> Response {
    sent(
        reach
            .services
            .screenshots
            .control
            .send(ShotCommand::Announce(path)),
        "screenshots",
    )
}

/// `sill debug corner`: a hit of `corner` on the first output, as if its dwell had run out.
pub fn corner(reach: &Reach, corner: CornerName) -> Response {
    let first = reach.outputs.outputs.first().map(|(info, _)| info.id);
    match first {
        Some(output) => {
            reach
                .links
                .corners
                .hit(crate::corners::corner_of(corner), output);
            Response::Done
        }
        None => unsupported("no output to fire a corner on"),
    }
}

/// `sill debug polkit`: a made-up request shown like a real one.
pub fn polkit(reach: &Reach, request: FakeRequest) -> Response {
    sent(
        reach
            .services
            .polkit_agent
            .control
            .send(PolkitCommand::Fake(request)),
        "polkit",
    )
}

/// `sill debug switcher`: the switcher on made-up apps, shown at once.
pub fn switcher(reach: &Reach, apps: Vec<AppId>) -> Response {
    switcher::ask(reach, SwitcherAsk::Fake(apps))
}

/// `sill debug auth`: a verdict for the lock screen's check. Only a daemon built with `debug`
/// takes it: an accepted verdict unlocks.
#[cfg(feature = "debug")]
pub fn auth(reach: &Reach, verdict: FakeVerdict) -> Response {
    use sill_model::auth::AuthCommand;
    sent(
        reach.services.auth.control.send(AuthCommand::Fake(verdict)),
        "auth",
    )
}

#[cfg(not(feature = "debug"))]
pub fn auth(_reach: &Reach, verdict: FakeVerdict) -> Response {
    unsupported(&format!(
        "built without the debug feature (a made-up {verdict:?} could unlock)"
    ))
}

/// `sill debug idle-wake`: a made-up `resumed`. Only a daemon built with `debug` takes it: a
/// wake inside the lock grace opens the lock.
#[cfg(feature = "debug")]
pub fn idle_wake(reach: &Reach) -> Response {
    use sill_model::idle::IdleCommand;
    sent(
        reach.services.idle.control.send(IdleCommand::FakeWake),
        "idle",
    )
}

#[cfg(not(feature = "debug"))]
pub fn idle_wake(_reach: &Reach) -> Response {
    unsupported("built without the debug feature (a made-up wake could open the lock)")
}
