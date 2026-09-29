//! The launch state's pure transition.

use super::{LaunchOutcome, Launches};

/// `launches` after `outcome`.
pub fn step(launches: Launches, outcome: LaunchOutcome) -> Launches {
    let Launches { mut scopes } = launches;
    match outcome {
        LaunchOutcome::Started(scope) => {
            if !scopes.iter().any(|s| s.unit == scope.unit) {
                scopes.push(scope);
            }
        }
        LaunchOutcome::Exited(unit) => scopes.retain(|s| s.unit != unit),
        LaunchOutcome::Failed { .. } | LaunchOutcome::Activated(_) => {}
    }
    Launches { scopes }
}
