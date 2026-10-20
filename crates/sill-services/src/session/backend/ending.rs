//! Whether a session action may go on: the ones that end the apps quit them first.

use sill_launcher::Confirm;
use sill_model::notifications::{
    AppRef, Body, Expiry, Hints, IconRef, NotificationsCommand, Posting,
};
use sill_model::session::{SessionAction, words};
use sill_model::windows::app_for;

use super::super::SessionLinks;
use super::super::quit_apps::{LiveApps, OpenWindow, QuitOutcome, Scopes, quit_apps};

/// The desktop entry the notice carries.
const ENTRY: &str = "sill-session";

/// Whether the action goes on to logind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ending {
    Proceed,
    Cancelled,
}

/// Quit the apps when `action` ends them (Log Out, Restart, Shut Down); `Cancelled`, with a
/// notice naming the app, when one keeps a window open. Lock and Sleep keep every app, so they
/// go on at once.
pub(super) async fn before(links: &SessionLinks, scopes: &Scopes, action: SessionAction) -> Ending {
    match action.confirm() {
        Confirm::Now => Ending::Proceed,
        Confirm::Ask => {
            let host = LiveApps {
                windows: links.windows.clone(),
                wl: links.wl.clone(),
                scopes: scopes.clone(),
            };
            match quit_apps(&host).await {
                QuitOutcome::Quit => Ending::Proceed,
                QuitOutcome::Held(windows) => {
                    tell(links, action, &windows);
                    Ending::Cancelled
                }
            }
        }
    }
}

/// Post the notice that the first of `windows` kept the session from ending.
fn tell(links: &SessionLinks, action: SessionAction, windows: &[OpenWindow]) {
    let Some(first) = windows.first() else { return };
    let name = {
        let index = links.apps.borrow();
        app_for(&first.app_id, &index).name(&index)
    };
    let (summary, body) = words::held(action, &name);
    let posting = Posting {
        replaces: None,
        app: AppRef {
            name: words::title(action).to_owned(),
            icon: IconRef::Name("system-log-out".to_owned()),
            desktop_entry: Some(ENTRY.to_owned()),
        },
        summary,
        body: Body::plain(&body),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    };
    // A stopped notifications service has nobody to tell.
    let _ = links
        .notifications
        .send(NotificationsCommand::Post(Box::new(posting)));
}
