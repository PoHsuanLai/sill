//! The app switcher on the daemon's side (M11 lane c; FINDINGS "M11 freeze" F528, F529): one
//! warm hidden switcher surface per output, `sill switcher next|prev|cancel|commit` written to
//! the [`SwitcherLink`] for the output the switcher opens on, and `sill debug switcher`.
//!
//! Which output: the one under the pointer (design/13) is not known to the daemon (F140), so
//! the main display (the banners' output) until a later pass knows the pointer's output.

use shell_host::{HostCtx, OutputId, OutputInfo, Root};
use sill_ipc::{FakeSwitcher, Response, SwitcherRequest};
use sill_launcher::AppId;
use sill_surfaces::surfaces::banners::BannerOutput;
use sill_surfaces::surfaces::switcher::{Dir, SwitcherAsk, SwitcherPanelProps, switcher_layer};

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden switcher for every output whose row lacks one.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.switcher.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = SwitcherPanelProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::SwitcherPanel, props));
        surfaces.switcher = crate::daemon::spawn_hidden(ctx, switcher_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The switcher's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    switcher_layer(info).namespace
}

/// The ask a request makes.
pub fn ask_of(request: SwitcherRequest) -> SwitcherAsk {
    match request {
        SwitcherRequest::Next => SwitcherAsk::Chord(Dir::Next),
        SwitcherRequest::Prev => SwitcherAsk::Chord(Dir::Prev),
        SwitcherRequest::Cancel => SwitcherAsk::Cancel,
        SwitcherRequest::Commit => SwitcherAsk::Commit,
    }
}

/// `sill switcher ...`.
pub fn dispatch(outputs: &Outputs, links: Option<&Links>, request: SwitcherRequest) -> Response {
    ask(outputs, links, ask_of(request))
}

/// `sill debug switcher APP...`.
pub fn fake(outputs: &Outputs, links: Option<&Links>, fake: FakeSwitcher) -> Response {
    let apps = fake.apps.into_iter().map(AppId).collect();
    ask(outputs, links, SwitcherAsk::Fake(apps))
}

fn ask(outputs: &Outputs, links: Option<&Links>, ask: SwitcherAsk) -> Response {
    let Some(links) = links else {
        return unsupported("the daemon's links are not installed");
    };
    match target(outputs, links.banners.now()) {
        Some(output) => {
            links.switcher.ask(output, ask);
            Response::Done
        }
        None => unsupported("no output has a switcher surface"),
    }
}

/// The output the switcher opens on: the banners' (the main display), else the first with a
/// switcher surface.
pub fn target(outputs: &Outputs, banners: BannerOutput) -> Option<OutputId> {
    let has_switcher = |id: OutputId| outputs.get(id).is_some_and(|s| s.switcher.is_some());
    match banners {
        BannerOutput::On(id) if has_switcher(id) => Some(id),
        _ => outputs
            .outputs
            .iter()
            .find(|(_, s)| s.switcher.is_some())
            .map(|(info, _)| info.id),
    }
}

fn unsupported(message: &str) -> Response {
    Response::Failed(sill_ipc::Failure {
        kind: sill_ipc::FailureKind::Unsupported,
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use sill_ipc::SwitcherRequest;
    use sill_surfaces::surfaces::switcher::{Dir, SwitcherAsk};

    use super::ask_of;

    #[test]
    fn requests_become_asks() {
        let cases = [
            (SwitcherRequest::Next, SwitcherAsk::Chord(Dir::Next)),
            (SwitcherRequest::Prev, SwitcherAsk::Chord(Dir::Prev)),
            (SwitcherRequest::Cancel, SwitcherAsk::Cancel),
            (SwitcherRequest::Commit, SwitcherAsk::Commit),
        ];
        for (request, want) in cases {
            assert_eq!(ask_of(request), want, "{request:?}");
        }
    }
}
