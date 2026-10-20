use sill_launcher::AppId;

use super::{
    DONE_KEPT, FailureCause, LaunchDone, LaunchOutcome, LaunchTiming, LaunchedScope, Launches, Pid,
    ScopeUnit, Stamp, answer_to, assistant_model, assistant_model_setup, launch_line, step,
};

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
        ..Launches::default()
    };
    let cases: Vec<(&str, Launches, LaunchOutcome, Launches)> = vec![
        (
            "started",
            Launches::default(),
            LaunchOutcome::Started(scope("foot", 1)),
            Launches {
                scopes: vec![scope("foot", 1)],
                ..Launches::default()
            },
        ),
        (
            "a duplicate start is one scope",
            Launches {
                scopes: vec![scope("foot", 1)],
                ..Launches::default()
            },
            LaunchOutcome::Started(scope("foot", 1)),
            Launches {
                scopes: vec![scope("foot", 1)],
                ..Launches::default()
            },
        ),
        (
            "exited",
            two.clone(),
            LaunchOutcome::Exited(scope("foot", 1).unit),
            Launches {
                scopes: vec![scope("foot", 2)],
                ..Launches::default()
            },
        ),
        (
            "failed changes nothing",
            two.clone(),
            LaunchOutcome::Failed {
                app: AppId("x".to_owned()),
                cause: FailureCause::CannotRun,
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

#[test]
fn the_model_pages_are_opened_by_the_ids_settings_serves() {
    assert_eq!(assistant_model().0, "page:assistant-model");
    assert_eq!(assistant_model_setup().0, "page:assistant-model-setup");
}

fn timing(
    app: &str,
    unit: Option<&str>,
    requested: u64,
    spawned: u64,
    pid: Option<u32>,
) -> LaunchTiming {
    LaunchTiming {
        app: AppId(app.to_owned()),
        unit: unit.map(|u| ScopeUnit(u.to_owned())),
        requested_at: Stamp(requested),
        spawned_at: Stamp(spawned),
        pid: pid.map(Pid),
    }
}

#[test]
fn the_timing_line_has_the_fields_scripts_read() {
    let cases: Vec<(&str, LaunchTiming, &str)> = vec![
        (
            "a scope",
            timing("foot", Some("app-sill-foot-1.scope"), 100, 250, Some(4242)),
            "launch desktop_id=foot unit=app-sill-foot-1.scope requested_at=100 spawned_at=250 pid=4242",
        ),
        (
            "activation has no unit and no process",
            timing("org.gnome.Calculator", None, 7, 9, None),
            "launch desktop_id=org.gnome.Calculator unit=- requested_at=7 spawned_at=9 pid=0",
        ),
        (
            "whitespace in an id becomes underscores",
            timing(
                "my app\tx",
                Some("app-sill-my\\x20app-1.scope"),
                1,
                2,
                Some(1),
            ),
            "launch desktop_id=my_app_x unit=app-sill-my\\x20app-1.scope requested_at=1 spawned_at=2 pid=1",
        ),
        (
            "a monotonic reading of days of uptime keeps every digit",
            timing(
                "foot",
                None,
                9_007_199_254_740_993,
                u64::MAX,
                Some(u32::MAX),
            ),
            "launch desktop_id=foot unit=- requested_at=9007199254740993 spawned_at=18446744073709551615 pid=4294967295",
        ),
    ];
    for (name, given, want) in cases {
        assert_eq!(launch_line(&given), want, "{name}");
    }
}

#[test]
fn a_clock_reading_is_whole_nanoseconds() {
    let cases = [
        ((0, 0), 0),
        ((3, 5), 3_000_000_005),
        ((86_400, 999_999_999), 86_400_999_999_999),
        ((u64::MAX, 1), u64::MAX),
    ];
    for ((seconds, nanos), want) in cases {
        assert_eq!(
            Stamp::of_reading(seconds, nanos),
            Stamp(want),
            "{seconds}.{nanos}"
        );
    }
}

#[test]
fn a_request_is_answered_by_the_first_end_of_its_app_at_or_after_it_was_made() {
    let refused = |app: &str, at: u64| LaunchDone::Refused {
        app: AppId(app.to_owned()),
        requested_at: Stamp(at),
        cause: FailureCause::CannotRun,
    };
    let spawned = |app: &str, at: u64| LaunchDone::Spawned(timing(app, None, at, at + 1, None));
    let done = vec![
        refused("foot", 10),
        spawned("other", 20),
        spawned("foot", 30),
    ];
    let cases: Vec<(&str, &str, u64, Option<&LaunchDone>)> = vec![
        ("an older end is not the answer", "foot", 11, Some(&done[2])),
        ("the end at the same instant is", "foot", 10, Some(&done[0])),
        ("another app's end is not", "other", 21, None),
        ("nothing yet", "foot", 31, None),
        ("an app never asked for", "kate", 0, None),
    ];
    for (name, app, asked_at, want) in cases {
        assert_eq!(
            answer_to(&done, &AppId(app.to_owned()), Stamp(asked_at)),
            want,
            "{name}"
        );
    }
}

#[test]
fn only_the_last_few_ends_are_kept() {
    let end = |n: u64| LaunchDone::Spawned(timing("foot", None, n, n, None));
    let mut launches = Launches::default();
    for n in 0..(DONE_KEPT as u64 + 3) {
        launches = step(launches, LaunchOutcome::Done(end(n)));
    }
    let kept: Vec<Stamp> = launches.done.iter().map(LaunchDone::requested_at).collect();
    let want: Vec<Stamp> = (3..DONE_KEPT as u64 + 3).map(Stamp).collect();
    assert_eq!(kept, want);
    assert!(launches.scopes.is_empty());
}
