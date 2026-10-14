//! The notification about a launch that failed: its posting, one per failed launch.

use sill_model::launch::{FailureCause, failure_text};
use sill_model::notifications::{
    AppRef, Body, Expiry, Hints, IconRef, NotificationsCommand, Posting,
};

use crate::control::Control;

/// The desktop entry the notices carry.
const ENTRY: &str = "sill-launch";

/// Tells the person that an app did not open.
#[derive(Debug, Clone)]
pub(crate) struct Tell(pub(crate) Control<NotificationsCommand>);

impl Tell {
    /// Post the notice that `app_name` did not open because of `cause`.
    pub(crate) fn failed(&self, app_name: &str, cause: FailureCause) {
        // A stopped notifications service has nobody to tell.
        let _ = self.0.send(NotificationsCommand::Post(Box::new(posting(
            app_name, cause,
        ))));
    }
}

fn posting(app_name: &str, cause: FailureCause) -> Posting {
    let notice = failure_text(app_name, cause);
    Posting {
        replaces: None,
        app: AppRef {
            name: "Apps".to_owned(),
            icon: IconRef::Name("dialog-warning".to_owned()),
            desktop_entry: Some(ENTRY.to_owned()),
        },
        summary: notice.summary,
        body: Body::plain(&notice.body),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}
