use std::sync::mpsc;
use std::time::Duration;

use sill_ipc::response::{Failure, FailureKind, Response};
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{
    FailureCause, LaunchCommand, LaunchDone, LaunchTiming, Launches, Pid, ScopeUnit, Stamp,
};
use sill_services::Control;
use sill_services::launch::monotonic_now;
use tokio::sync::{oneshot, watch};

use super::{LaunchAsk, response_of, serve, unknown_app};

fn app(id: &str) -> AppId {
    AppId(id.to_owned())
}

fn timing_at(requested_at: Stamp) -> LaunchTiming {
    LaunchTiming {
        app: app("foot"),
        unit: Some(ScopeUnit("app-sill-foot-1.scope".to_owned())),
        requested_at,
        spawned_at: Stamp(requested_at.0 + 1),
        pid: Some(Pid(7)),
    }
}

#[test]
fn each_end_of_a_launch_is_the_answer_sill_launch_reads() {
    let cases = [
        (
            "spawned",
            LaunchDone::Spawned(timing_at(Stamp(5))),
            Response::Launched(timing_at(Stamp(5))),
        ),
        (
            "not installed",
            LaunchDone::Refused {
                app: app("foot"),
                requested_at: Stamp(5),
                cause: FailureCause::NotInstalled,
            },
            Response::Failed(Failure {
                kind: FailureKind::UnknownApp,
                message: "Foot could not be opened because it is not in your apps any more."
                    .to_owned(),
            }),
        ),
        (
            "cannot run",
            LaunchDone::Refused {
                app: app("foot"),
                requested_at: Stamp(5),
                cause: FailureCause::CannotRun,
            },
            Response::Failed(Failure {
                kind: FailureKind::Refused,
                message: "Foot could not be opened because its program could not be run."
                    .to_owned(),
            }),
        ),
    ];
    for (name, done, want) in cases {
        assert_eq!(response_of(&done, "Foot"), want, "{name}");
    }
}

#[test]
fn an_id_no_app_has_is_refused_in_plain_words() {
    assert_eq!(
        unknown_app(&app("nope")),
        Response::Failed(Failure {
            kind: FailureKind::UnknownApp,
            message: "There is no installed app with the id nope.".to_owned(),
        })
    );
}

/// The service stood in for by a task that publishes the end of the one launch it is told.
#[tokio::test]
async fn a_launch_carries_the_docks_token_and_is_answered_with_its_timing() {
    let (control, mut commands) = Control::<LaunchCommand>::channel();
    let (published, state) = watch::channel(Launches::default());
    let service = tokio::spawn(async move {
        let command = commands.recv().await;
        let requested_at = monotonic_now();
        published.send_modify(|launches| {
            launches
                .done
                .push(LaunchDone::Spawned(timing_at(requested_at)));
        });
        command
    });
    let (token, granted) = oneshot::channel();
    token
        .send(Some("tok".to_owned()))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let (reply, answer) = mpsc::sync_channel(1);

    serve(
        LaunchAsk {
            app: app("foot"),
            name: "Foot".to_owned(),
            token: granted,
            reply,
        },
        control,
        state,
    )
    .await;

    let Some(LaunchCommand::Launch(request)) = service.await.unwrap_or_else(|e| panic!("{e}"))
    else {
        panic!("the service was told something else");
    };
    assert_eq!(request.app, app("foot"));
    assert_eq!(request.action, DesktopAction::Default);
    assert_eq!(request.token.as_deref(), Some("tok"));
    assert!(request.files.is_empty() && request.uris.is_empty());
    let response = answer
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(matches!(response, Response::Launched(t) if t.app == app("foot")));
}

#[tokio::test]
async fn a_launch_of_a_stopped_service_is_told_so() {
    let (control, commands) = Control::<LaunchCommand>::channel();
    drop(commands);
    let (_published, state) = watch::channel(Launches::default());
    let (_never, token) = oneshot::channel();
    let (reply, answer) = mpsc::sync_channel(1);

    serve(
        LaunchAsk {
            app: app("foot"),
            name: "Foot".to_owned(),
            token,
            reply,
        },
        control,
        state,
    )
    .await;

    let Ok(Response::Failed(failure)) = answer.try_recv() else {
        panic!("no refusal");
    };
    assert_eq!(failure.kind, FailureKind::Internal);
    assert!(
        failure.message.starts_with("Foot could not be opened"),
        "{}",
        failure.message
    );
}
