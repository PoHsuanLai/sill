use sill_launcher::AppId;

use super::{LaunchOutcome, LaunchedScope, Launches, ScopeUnit, step};

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
        assert_eq!(step(before, outcome), want, "{name}");
    }
}
