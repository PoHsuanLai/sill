//! sill's own polkit agent on the daemon's side (M11 lane b; FINDINGS "M11 freeze" F526): the
//! agent's policy from `session.polkit_agent` ([`spawn_policy`]: `Own` only for `sill`; the
//! service stays passive until the first policy), one warm hidden prompt surface per output,
//! `sill polkit status`, and `sill debug polkit`.
//!
//! Registering an agent is per logind session: nested under KDE, the daemon shares KDE's
//! session, whose agent (polkit-kde) already holds it, so polkitd refuses sill's
//! (`Registration::Refused`), and that is the right outcome there. Acceptance runs the agent on
//! a private bus with a stand-in Authority (F527), never on the system bus.

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{
    Failure, FailureKind, FakePolkit, PolkitReport, PolkitRequest, ReportRegistration, Response,
};
use sill_services::polkit::AgentChoice;
use sill_services::polkit_agent::{
    ActionId, AgentPolicy, FakeRequest, Polkit, PolkitCommand, Registration,
};
use sill_services::{Control, Services};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::polkit_prompt::{PolkitPromptProps, polkit_layer};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// The agent's policy `file` makes: `Own` exactly when `session.polkit_agent` is `sill`.
pub fn policy_of(file: &ShellFile) -> AgentPolicy {
    match AgentChoice::parse(&file.session.polkit_agent) {
        AgentChoice::Own => AgentPolicy::Own,
        AgentChoice::Auto | AgentChoice::Off | AgentChoice::Known(_) | AgentChoice::Command(_) => {
            AgentPolicy::Off
        }
    }
}

/// Send [`policy_of`] now and on every settings change that changes it, until either side is
/// gone (the notifications policy's pattern).
pub fn spawn_policy(
    rt: &Handle,
    mut settings: watch::Receiver<ShellFile>,
    control: Control<PolkitCommand>,
) {
    rt.spawn(async move {
        let mut sent = None;
        loop {
            let policy = policy_of(&settings.borrow_and_update());
            if sent != Some(policy) {
                if control.send(PolkitCommand::Policy(policy)).is_err() {
                    return;
                }
                sent = Some(policy);
            }
            if settings.changed().await.is_err() {
                return;
            }
        }
    });
}

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
        PolkitRequest::Status => Response::Polkit(report_of(&services.polkit.borrow())),
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

/// `sill debug polkit`: a made-up request through the same reducer (lane b's live loop answers
/// it with its in-process stand-in helper).
pub fn fake(services: &Services, fake: FakePolkit) -> Response {
    let request = FakeRequest {
        action_id: ActionId(fake.action_id),
        message: fake.message,
        icon: String::new(),
    };
    match services.polkit_control.send(PolkitCommand::Fake(request)) {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: format!("polkit: {error}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use sill_services::polkit_agent::{AgentPolicy, Polkit, Registration};
    use sill_settings::ShellFile;

    use super::{policy_of, report_of};

    #[test]
    fn only_sill_makes_the_agent_ours() {
        let with = |agent: &str| {
            let mut file = ShellFile::default();
            file.session.polkit_agent = agent.to_owned();
            policy_of(&file)
        };
        assert_eq!(with("sill"), AgentPolicy::Own);
        for agent in ["auto", "off", "cosmic-osd", "my-agent --flag"] {
            assert_eq!(with(agent), AgentPolicy::Off, "{agent}");
        }
        assert_eq!(policy_of(&ShellFile::default()), AgentPolicy::Off);
    }

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
