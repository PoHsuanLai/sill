//! The single-instance lock file, watched. The lock belongs to a file, and a cache cleaner or a
//! tidy-minded person can delete that file: the next `sill` would lock a new one and start a second
//! daemon beside this one, which then unlinks this daemon's socket for its own. latchkey's rule for
//! a daemon is to ask `standing()` now and then and to stop when it is displaced; [`spawn`] is that
//! question, asked on the daemon's async runtime every [`STANDING_EVERY`].
//!
//! A displaced shell does what a shell whose bus name was taken does ([`crate::name_guard`]): says
//! so in one line, ends the host and hands the daemon the line, which becomes its failure. The
//! unit restarts the shell a second later, and the new one takes the lock file that is there now
//! (or exits at once if a rival already holds it).

use std::sync::mpsc;
use std::time::Duration;

use latchkey::lock::Standing;
use shell_host::HostHandle;
use tokio::runtime::Handle;
use tokio::time::MissedTickBehavior;

use crate::daemon::msg::ShellMsg;
use crate::ipc::door::Door;

/// How often the lock file is looked at: latchkey says once a minute is plenty, and a second
/// daemon that starts in the gap is found within the minute and ends this one.
pub const STANDING_EVERY: Duration = Duration::from_secs(60);

/// The line the shell stops with.
pub fn line() -> String {
    "the file that keeps this shell the only one was removed, so another could start beside it; \
     this shell stops, and starts again"
        .to_owned()
}

/// What to do about a standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Stay,
    StepAside,
}

/// The decision latchkey prescribes: only a lock file that is still ours lets the daemon stay.
pub fn verdict(standing: Standing) -> Verdict {
    match standing {
        Standing::Ours => Verdict::Stay,
        _ => Verdict::StepAside,
    }
}

/// Look at `door` now and then, and return the line to stop with the first time the daemon is
/// displaced. The first look is at once, so a file gone before this ran is not missed.
pub async fn until_displaced(door: Door, every: Duration) -> String {
    let mut ticks = tokio::time::interval(every);
    // A look that ran late is not made up for: one look per interval is all that is asked.
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        // Two file lookups, once a minute: cheaper than handing them to the blocking pool.
        if verdict(door.standing()) == Verdict::StepAside {
            return line();
        }
    }
}

/// Watch `door` on `rt` until the daemon is displaced: then send the line on `stopped` and end the
/// host. No thread of its own.
pub fn spawn(rt: &Handle, door: Door, host: HostHandle<ShellMsg>, stopped: mpsc::Sender<String>) {
    rt.spawn(async move {
        let said = until_displaced(door, STANDING_EVERY).await;
        let _ = stopped.send(said);
        let _ = host.exit();
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use latchkey::lock::Standing;
    use sill_testkit::TempDir;

    use super::{Verdict, line, until_displaced, verdict};
    use crate::ipc::door::Door;

    const QUICK: Duration = Duration::from_millis(10);

    fn held_in(dir: &TempDir) -> (latchkey::Agent, Door) {
        let env = latchkey::Environment {
            runtime_dir: Some(dir.path().as_os_str()),
            ..Default::default()
        };
        let agent = latchkey::Agent::in_environment("guard", latchkey::here(), &env)
            .unwrap_or_else(|e| panic!("building the agent: {e}"));
        let listening = agent.listen().unwrap_or_else(|e| panic!("listening: {e}"));
        (agent, Door::from(listening))
    }

    #[test]
    fn only_a_lock_file_that_is_ours_lets_the_daemon_stay() {
        let cases = [
            ("ours", Standing::Ours, Verdict::Stay),
            ("displaced", Standing::Displaced, Verdict::StepAside),
        ];
        for (name, standing, want) in cases {
            assert_eq!(verdict(standing), want, "{name}");
        }
    }

    #[tokio::test]
    async fn a_removed_lock_file_ends_the_watch_with_the_line() {
        let dir = TempDir::new("door-guard-removed");
        let (agent, door) = held_in(&dir);
        let watching = tokio::spawn(until_displaced(door, QUICK));
        // Several looks go by while the file is there: the watch keeps going.
        tokio::time::sleep(QUICK * 5).await;
        assert!(
            !watching.is_finished(),
            "ended while the lock file was ours"
        );
        std::fs::remove_file(&agent.address().lock).unwrap_or_else(|e| panic!("{e}"));
        let said = tokio::time::timeout(Duration::from_secs(5), watching)
            .await
            .unwrap_or_else(|_| panic!("the watch never noticed the removed file"))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(said, line());
    }

    #[tokio::test]
    async fn a_lock_file_replaced_by_another_ends_the_watch() {
        let dir = TempDir::new("door-guard-replaced");
        let (agent, door) = held_in(&dir);
        // A cleaner removed it and a second `sill` made a new one in its place.
        std::fs::remove_file(&agent.address().lock).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&agent.address().lock, b"").unwrap_or_else(|e| panic!("{e}"));
        let said = tokio::time::timeout(Duration::from_secs(5), until_displaced(door, QUICK))
            .await
            .unwrap_or_else(|_| panic!("a new file at the path was taken for ours"));
        assert_eq!(said, line());
    }
}
