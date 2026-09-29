//! sill's own polkit agent on the daemon's side: one warm hidden prompt surface per output,
//! `sill polkit status`, and `sill debug polkit`. The daemon tells the service to register at
//! start; it stays passive until then.
//!
//! Registering an agent is per logind session: nested in another desktop's session, the daemon
//! shares that session, whose own agent already holds it, so polkitd refuses sill's
//! (`Registration::Refused`), and that is the right outcome there. Acceptance runs the agent on
//! a private bus with a stand-in Authority, never on the system bus.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{
    Failure, FailureKind, FakePolkit, PolkitReport, PolkitRequest, ReportRegistration, Response,
};
use sill_model::polkit_agent::{ActionId, FakeRequest, Polkit, PolkitCommand, Registration};
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::polkit_prompt::{PolkitPromptProps, polkit_layer};

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden polkit prompt for every output whose row lacks one (after the power menu, so
/// on the Overlay layer the prompt draws over it).
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.polkit_prompt.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        let props = PolkitPromptProps {
            output: info.clone(),
        };
        let root = root(Root::with_props(sill_surfaces::PolkitPrompt, props));
        surfaces.polkit_prompt = crate::daemon::spawn_hidden(ctx, polkit_layer(&info), root, &info);
        outputs.set(info, surfaces);
    }
}

/// The prompt's namespace on `info`, which `debug stats` reports.
pub fn namespace(info: &OutputInfo) -> String {
    polkit_layer(info).namespace
}

/// `sill polkit status`.
pub fn dispatch(services: &Services, request: PolkitRequest) -> Response {
    match request {
        PolkitRequest::Status => Response::Polkit(report_of(&services.polkit_agent.state.borrow())),
    }
}

/// The agent's state as the wire's.
pub fn report_of(polkit: &Polkit) -> PolkitReport {
    PolkitReport {
        registration: match &polkit.registration {
            Registration::Passive => ReportRegistration::Passive,
            Registration::NoBus => ReportRegistration::NoBus,
            Registration::Registered => ReportRegistration::Registered,
            Registration::Refused(message) => ReportRegistration::Refused(message.clone()),
        },
        showing: polkit.current.as_ref().map(|view| view.action_id.0.clone()),
        waiting: polkit.waiting,
    }
}

/// `sill debug polkit`: a made-up request through the same reducer (the live loop answers it with
/// its in-process stand-in helper).
pub fn fake(services: &Services, fake: FakePolkit) -> Response {
    let request = FakeRequest {
        action_id: ActionId(fake.action_id),
        message: fake.message,
        icon: String::new(),
    };
    match services
        .polkit_agent
        .control
        .send(PolkitCommand::Fake(request))
    {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: format!("polkit: {error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use sill_model::polkit_agent::{Polkit, Registration};

    use super::report_of;

    #[test]
    fn a_refusal_is_reported() {
        let report = report_of(&Polkit {
            registration: Registration::Refused("an agent is registered".to_owned()),
            current: None,
            waiting: 0,
        });
        assert_eq!(
            report.registration,
            sill_ipc::ReportRegistration::Refused("an agent is registered".to_owned())
        );
        assert_eq!(report.showing, None);
    }
}
