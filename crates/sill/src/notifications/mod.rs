//! The daemon's notifications half: one banner column and one notification center per output,
//! spawned warm and hidden (the OSD's and the power menu's pattern); the banners' output; the
//! `notifications` requests, which write the shared [`crate::links`] or tell the service; `sill
//! debug notify`; and the service's policy, followed from the settings and the focus mode
//! ([`spawn_policy`]).

mod fake;
mod policy;

pub use fake::posting_of;
pub use policy::{policy_of, spawn_policy};

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{
    FakeNotification, NotificationsReport, NotificationsRequest, ReportHolder, ReportMode, Response,
};
use sill_services::{
    History, NameHolder, Notifications, NotificationsCommand, ServerMode, Services,
};
use sill_settings::ShellFile;
use sill_surfaces::surfaces::banners::{BannerOutput, BannersProps, banners_layer};
use sill_surfaces::surfaces::notification_center::{NotificationCenterProps, center_layer};
use tokio::sync::watch;

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden banner column and a hidden notification center for every output whose row
/// lacks one, each root built by `root` (the daemon's `surface_root`, which adds the links).
/// Called before the OSD's `spawn_missing`, so the OSD draws over a banner on the Overlay layer.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    settings: &watch::Sender<ShellFile>,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, s)| s.banners.is_none() || s.notification_center.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let file = settings.borrow().clone();
    for info in missing {
        let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
        if surfaces.banners.is_none() {
            let props = BannersProps {
                output: info.clone(),
            };
            let root = root(Root::with_props(sill_surfaces::Banners, props));
            let config = banners_layer(&info, &file.notifications);
            surfaces.banners = crate::daemon::spawn_hidden(ctx, config, root, &info);
        }
        if surfaces.notification_center.is_none() {
            let props = NotificationCenterProps {
                output: info.clone(),
            };
            let root = root(Root::with_props(sill_surfaces::NotificationCenter, props));
            surfaces.notification_center =
                crate::daemon::spawn_hidden(ctx, center_layer(&info), root, &info);
        }
        outputs.set(info, surfaces);
    }
}

/// The output the banners show on: the first with a banner column (the main display's
/// stand-in, `banners/placement.rs`).
pub fn banner_output(outputs: &Outputs) -> BannerOutput {
    outputs
        .outputs
        .iter()
        .find(|(_, s)| s.banners.is_some())
        .map_or(BannerOutput::Nowhere, |(info, _)| BannerOutput::On(info.id))
}

/// `sill notifications ...`: the center's requests write its link, on the first output with a
/// center surface; `dismiss-all` and `status` go to the service.
pub fn dispatch(
    outputs: &Outputs,
    links: Option<&Links>,
    services: &Services,
    request: NotificationsRequest,
) -> Response {
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.notification_center.is_some())
        .map(|(info, _)| info.id);
    let center = links.map(|links| &links.notification_center);
    match (request, center, target) {
        (NotificationsRequest::DismissAll, _, _) => {
            send(services, NotificationsCommand::DismissAll)
        }
        (NotificationsRequest::Status, _, _) => {
            let notes = services.notifications.borrow();
            Response::Notifications(report_of(&notes.live, &notes.history))
        }
        (NotificationsRequest::Close, Some(center), _) => {
            center.close();
            Response::Done
        }
        (NotificationsRequest::Toggle, Some(center), Some(output)) => {
            center.toggle(output);
            Response::Done
        }
        (NotificationsRequest::Open, Some(center), Some(output)) => {
            center.open(output);
            Response::Done
        }
        (NotificationsRequest::Close, None, _)
        | (NotificationsRequest::Toggle | NotificationsRequest::Open, _, _) => {
            unsupported("no output has a notification center surface")
        }
    }
}

/// `sill debug notify`: post `fake` to the service, as if `Notify` had been called.
pub fn fake(services: &Services, fake: FakeNotification) -> Response {
    send(
        services,
        NotificationsCommand::Post(Box::new(posting_of(fake))),
    )
}

/// What `sill notifications status` prints.
pub fn report_of(live: &Notifications, history: &History) -> NotificationsReport {
    NotificationsReport {
        mode: match live.server.mode {
            ServerMode::Own => ReportMode::Own,
            ServerMode::Passive => ReportMode::Passive,
        },
        holder: match &live.server.holder {
            NameHolder::NoBus => ReportHolder::NoBus,
            NameHolder::Nobody => ReportHolder::Nobody,
            NameHolder::Sill => ReportHolder::Sill,
            NameHolder::Other(owner) => ReportHolder::Other {
                unique: owner.unique.clone(),
                pid: owner.pid,
                command: owner.command.clone(),
            },
        },
        banners: u32::try_from(live.banners.len()).unwrap_or(u32::MAX),
        kept: u32::try_from(history.len()).unwrap_or(u32::MAX),
    }
}

/// The two surfaces' namespaces on `info`, which `debug stats` reports.
pub fn namespaces(info: &OutputInfo) -> (String, String) {
    (
        banners_layer(info, &Default::default()).namespace,
        center_layer(info).namespace,
    )
}

fn send(services: &Services, command: NotificationsCommand) -> Response {
    match services.notifications_control.send(command) {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(sill_ipc::Failure {
            kind: sill_ipc::FailureKind::Internal,
            message: format!("notifications: {error}"),
        }),
    }
}

fn unsupported(message: &str) -> Response {
    Response::Failed(sill_ipc::Failure {
        kind: sill_ipc::FailureKind::Unsupported,
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests;
