//! The scopes we started, and their one reducer: the dock stops a launch bounce when the scope
//! exits (design/10 §10.3.5), and Force Quit needs the unit.

use serde::{Deserialize, Serialize};
use sill_launcher::AppId;

/// A transient systemd scope's unit name: `app-sill-<id>-<nonce>.scope`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeUnit(pub String);

/// Every scope we started that has not exited, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Launches {
    pub scopes: Vec<LaunchedScope>,
}

/// One running scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchedScope {
    pub app: AppId,
    pub unit: ScopeUnit,
}

/// What became of a launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchOutcome {
    Started(LaunchedScope),
    Exited(ScopeUnit),
    /// The launch did not happen; the dock stops its bounce.
    Failed {
        app: AppId,
        reason: String,
    },
    /// D-Bus activation: no scope of ours to track.
    Activated(AppId),
}

/// `launches` after `outcome`.
pub fn reduce_launches(launches: Launches, outcome: LaunchOutcome) -> Launches {
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

#[cfg(test)]
mod tests {
    use super::{LaunchOutcome, LaunchedScope, Launches, ScopeUnit, reduce_launches};
    use sill_launcher::AppId;

    fn scope(app: &str, n: u32) -> LaunchedScope {
        LaunchedScope {
            app: AppId(app.to_owned()),
            unit: ScopeUnit(format!("app-sill-{app}-{n}.scope")),
        }
    }

    #[test]
    fn scopes_come_and_go() {
        let two = Launches {
            scopes: vec![scope("foot", 1), scope("foot", 2)],
        };
        let cases: Vec<(&str, Launches, LaunchOutcome, Launches)> = vec![
            (
                "started",
                Launches::default(),
                LaunchOutcome::Started(scope("foot", 1)),
                Launches {
                    scopes: vec![scope("foot", 1)],
                },
            ),
            (
                "a duplicate start is one scope",
                Launches {
                    scopes: vec![scope("foot", 1)],
                },
                LaunchOutcome::Started(scope("foot", 1)),
                Launches {
                    scopes: vec![scope("foot", 1)],
                },
            ),
            (
                "exited",
                two.clone(),
                LaunchOutcome::Exited(scope("foot", 1).unit),
                Launches {
                    scopes: vec![scope("foot", 2)],
                },
            ),
            (
                "failed changes nothing",
                two.clone(),
                LaunchOutcome::Failed {
                    app: AppId("x".to_owned()),
                    reason: "no exec".to_owned(),
                },
                two.clone(),
            ),
            (
                "activated changes nothing",
                two.clone(),
                LaunchOutcome::Activated(AppId("x".to_owned())),
                two.clone(),
            ),
        ];
        for (name, before, outcome, want) in cases {
            assert_eq!(reduce_launches(before, outcome), want, "{name}");
        }
    }
}
