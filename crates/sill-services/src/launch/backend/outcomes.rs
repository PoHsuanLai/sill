//! Where a launch's outcomes go: into the published state, a failure also to the person, and the
//! timing line of a launch that reached an app to the journal.

use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_model::launch::{
    FailureCause, LaunchDone, LaunchOutcome, LaunchTiming, Launches, Stamp, launch_line, step,
};
use tokio::sync::watch;

use super::notice::Tell;

/// Where outcomes go: into the published state, and a failure also to the person.
#[derive(Clone)]
pub(super) struct Outcomes {
    pub(super) tx: watch::Sender<Launches>,
    pub(super) tell: Tell,
    pub(super) apps: watch::Receiver<AppIndex>,
}

impl Outcomes {
    pub(super) fn record(&self, outcome: LaunchOutcome) {
        if let LaunchOutcome::Failed { app, cause } = &outcome {
            let name = self
                .apps
                .borrow()
                .get(app)
                .map_or_else(|| app.0.clone(), |found| found.name.clone());
            self.tell.failed(&name, *cause);
        }
        record(&self.tx, outcome);
    }

    /// A launch request that reached an app: its timing line goes to the journal, and its end to
    /// whoever asked.
    pub(super) fn spawned(&self, timing: LaunchTiming) {
        eprintln!("timing: {}", launch_line(&timing));
        self.record(LaunchOutcome::Done(LaunchDone::Spawned(timing)));
    }

    /// A launch request that did not happen: the person is told `cause`; `detail` goes to the
    /// journal.
    pub(super) fn refused(
        &self,
        app: AppId,
        requested_at: Stamp,
        cause: FailureCause,
        detail: &str,
    ) {
        self.record(failed(app.clone(), cause, detail));
        self.record(LaunchOutcome::Done(LaunchDone::Refused {
            app,
            requested_at,
            cause,
        }));
    }
}

/// A failed launch of `app`: the person is told `cause`; `detail` goes to the journal.
pub(super) fn failed(app: AppId, cause: FailureCause, detail: &str) -> LaunchOutcome {
    eprintln!("sill: launch: {}: {detail}", app.0);
    LaunchOutcome::Failed { app, cause }
}

fn record(tx: &watch::Sender<Launches>, outcome: LaunchOutcome) {
    tx.send_modify(|launches| *launches = step(std::mem::take(launches), outcome));
}
