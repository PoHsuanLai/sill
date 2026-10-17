//! The launch state's pure transition.

use super::model::DONE_KEPT;
use super::{LaunchOutcome, Launches};

/// `launches` after `outcome`.
pub fn step(launches: Launches, outcome: LaunchOutcome) -> Launches {
    let Launches {
        mut scopes,
        mut done,
    } = launches;
    match outcome {
        LaunchOutcome::Started(scope) => {
            if !scopes.iter().any(|s| s.unit == scope.unit) {
                scopes.push(scope);
            }
        }
        LaunchOutcome::Exited(unit) => scopes.retain(|s| s.unit != unit),
        LaunchOutcome::Done(entry) => {
            done.push(entry);
            let extra = done.len().saturating_sub(DONE_KEPT);
            done.drain(..extra);
        }
        LaunchOutcome::Failed { .. } | LaunchOutcome::Activated(_) => {}
    }
    Launches { scopes, done }
}
