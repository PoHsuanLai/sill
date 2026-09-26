//! One helper run for the request on screen: polkit's helper as a `kill_on_drop` child or on
//! its socket, or, for a `sill debug polkit` request, an in-process stand-in that accepts
//! [`FAKE_SECRET`]. A run reports its lines as [`RunEvent`]s tagged with its [`RunId`], so a
//! stopped run's late lines are told apart; dropping a [`Run`] aborts its task, which kills the
//! child. The answer reaches the task over an in-process channel and is written from a buffer
//! that is zeroed when dropped; it is never logged.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::auth::{Secret, UserName};

use super::helper::{HelperLine, HelperRoute, opening, parse_helper_line};
use super::types::{Cookie, FAKE_SECRET};

/// Which run a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct RunId(pub u64);

/// What a run said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RunSaid {
    Line(HelperLine),
    Ended,
}

/// One thing a run said, tagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RunEvent {
    pub cookie: Cookie,
    pub run: RunId,
    pub said: RunSaid,
}

/// Who answers a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Driver {
    /// polkit's helper (or a stand-in program) by this route.
    Helper(HelperRoute),
    /// The in-process stand-in of a made-up request.
    StandIn,
}

/// A running helper.
#[derive(Debug)]
pub(super) struct Run {
    pub cookie: Cookie,
    pub id: RunId,
    answers: mpsc::UnboundedSender<Secret>,
    task: JoinHandle<()>,
}

impl Run {
    /// Hand the answer to the run (dropped, and so zeroed, when the run is gone).
    pub fn answer(&self, secret: Secret) {
        let _ = self.answers.send(secret);
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Where a run's events go.
#[derive(Debug, Clone)]
pub(super) struct Report {
    cookie: Cookie,
    run: RunId,
    events: mpsc::UnboundedSender<RunEvent>,
}

impl Report {
    fn say(&self, said: RunSaid) {
        let _ = self.events.send(RunEvent {
            cookie: self.cookie.clone(),
            run: self.run,
            said,
        });
    }
}

/// Start run `id` for `cookie` as `user`.
pub(super) fn start(
    driver: &Driver,
    cookie: Cookie,
    user: &UserName,
    id: RunId,
    events: mpsc::UnboundedSender<RunEvent>,
) -> Run {
    let (answers, answer_rx) = mpsc::unbounded_channel();
    let report = Report {
        cookie: cookie.clone(),
        run: id,
        events,
    };
    let task = match driver {
        Driver::StandIn => tokio::spawn(stand_in(report, answer_rx)),
        Driver::Helper(route) => {
            let (route, user, cookie) = (route.clone(), user.clone(), cookie.clone());
            tokio::spawn(async move {
                let _ = helper(&route, &user, &cookie, &report, answer_rx).await;
                report.say(RunSaid::Ended);
            })
        }
    };
    Run {
        cookie,
        id,
        answers,
        task,
    }
}

/// How long the stand-in takes to check, so `Checking` is seen as it would be with PAM.
const STAND_IN_CHECK: Duration = Duration::from_millis(300);

async fn stand_in(report: Report, mut answers: mpsc::UnboundedReceiver<Secret>) {
    report.say(RunSaid::Line(HelperLine::PromptHidden(
        "Password: ".to_owned(),
    )));
    let Some(secret) = answers.recv().await else {
        return;
    };
    tokio::time::sleep(STAND_IN_CHECK).await;
    let line = if secret.expose() == FAKE_SECRET {
        HelperLine::Success
    } else {
        HelperLine::Failure
    };
    drop(secret);
    report.say(RunSaid::Line(line));
}

async fn helper(
    route: &HelperRoute,
    user: &UserName,
    cookie: &Cookie,
    report: &Report,
    answers: mpsc::UnboundedReceiver<Secret>,
) -> std::io::Result<()> {
    let Some(opening) = opening(route, user, cookie) else {
        return Ok(());
    };
    match route {
        HelperRoute::Socket(path) => {
            let (read, write) = tokio::net::UnixStream::connect(path).await?.into_split();
            talk(BufReader::new(read), write, &opening.lines, report, answers).await
        }
        HelperRoute::Setuid(program) => {
            let mut child = tokio::process::Command::new(program)
                .args(&opening.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()?;
            let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
                return Ok(());
            };
            talk(
                BufReader::new(stdout),
                stdin,
                &opening.lines,
                report,
                answers,
            )
            .await?;
            child.wait().await.map(|_| ())
        }
        HelperRoute::Missing => Ok(()),
    }
}

/// Write the opening, then relay lines out and answers in until the helper closes its end.
async fn talk(
    mut read: impl AsyncBufRead + Unpin,
    mut write: impl AsyncWrite + Unpin,
    opening: &[String],
    report: &Report,
    mut answers: mpsc::UnboundedReceiver<Secret>,
) -> std::io::Result<()> {
    for line in opening {
        write.write_all(format!("{line}\n").as_bytes()).await?;
    }
    write.flush().await?;
    let mut line = String::new();
    loop {
        tokio::select! {
            read_now = read.read_line(&mut line) => {
                if read_now? == 0 {
                    return Ok(());
                }
                let text = line.trim_end_matches(['\n', '\r']);
                if let Ok(parsed) = parse_helper_line(text) {
                    report.say(RunSaid::Line(parsed));
                }
                line.clear();
            }
            Some(secret) = answers.recv() => write_answer(&mut write, secret).await?,
        }
    }
}

/// The answer and its newline, in a buffer zeroed when dropped (also on an error).
async fn write_answer(
    write: &mut (impl AsyncWrite + Unpin),
    secret: Secret,
) -> std::io::Result<()> {
    let mut bytes = Wiped(Vec::with_capacity(secret.expose().len() + 1));
    bytes.0.extend_from_slice(secret.expose().as_bytes());
    bytes.0.push(b'\n');
    drop(secret);
    write.write_all(&bytes.0).await?;
    write.flush().await
}

/// Bytes overwritten with zeros when dropped.
struct Wiped(Vec<u8>);

impl Drop for Wiped {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|byte| *byte = 0);
        std::hint::black_box(&self.0);
    }
}
