//! The shell's names on the session bus. `org.quire.Shell` (the shell's own actions) and
//! `org.quire.Confirm1` (the confirmation sheet) are asked for without queueing: a program that
//! already owns either is not waited for, and the shell does not run beside it. The portal asks the
//! person through the owner of `org.quire.Shell`, so a program that took the name during a restart
//! must not be left holding it while the shell goes on without. [`spawn`] watches the seat and,
//! when a name is taken, says so in one line, ends the host and hands the daemon the line, which
//! becomes its failure. The unit restarts the shell a second later, and the name is asked for
//! again.

use std::sync::mpsc;

use shell_host::HostHandle;
use sill_model::intents::{Intents, IntentsLink};
use sill_model::serve::ClaimError;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

/// What the bus said, when the seat could not take a name because another program owns it.
pub fn taken(link: &IntentsLink) -> Option<&str> {
    match link {
        IntentsLink::Unavailable(ClaimError::NameTaken(said)) => Some(said),
        IntentsLink::Passive
        | IntentsLink::Serving
        | IntentsLink::Unavailable(ClaimError::NoBus | ClaimError::Refused(_)) => None,
    }
}

/// The line the shell stops with.
pub fn line(said: &str) -> String {
    format!(
        "another program already owns a name on the session bus that only the shell may hold \
         ({said}); this shell stops. Quit that program, and the shell starts again"
    )
}

/// Waits until `intents` says a name is taken, and returns the line to stop with; `None` when
/// the service is gone. Looks at the state as it is now first, so a name taken before this ran is
/// not missed.
pub async fn until_taken(mut intents: watch::Receiver<Intents>) -> Option<String> {
    loop {
        let said = taken(&intents.borrow_and_update().link).map(line);
        if said.is_some() {
            return said;
        }
        intents.changed().await.ok()?;
    }
}

/// Watch `intents` on `rt` until a name is taken: then send the line on `stopped` and end the
/// host.
pub fn spawn(
    rt: &Handle,
    intents: watch::Receiver<Intents>,
    host: HostHandle<ShellMsg>,
    stopped: mpsc::Sender<String>,
) {
    rt.spawn(async move {
        if let Some(said) = until_taken(intents).await {
            let _ = stopped.send(said);
            let _ = host.exit();
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_model::intents::{Intents, IntentsLink};
    use sill_model::serve::ClaimError;
    use tokio::sync::watch;

    use super::{line, taken, until_taken};

    #[test]
    fn only_a_name_that_is_taken_stops_the_shell() {
        let cases = [
            ("not asked yet", IntentsLink::Passive, None),
            ("serving", IntentsLink::Serving, None),
            ("no bus", IntentsLink::Unavailable(ClaimError::NoBus), None),
            (
                "the bus refused",
                IntentsLink::Unavailable(ClaimError::Refused("no".to_owned())),
                None,
            ),
            (
                "taken",
                IntentsLink::Unavailable(ClaimError::NameTaken(
                    "org.quire.Shell is taken".to_owned(),
                )),
                Some("org.quire.Shell is taken"),
            ),
        ];
        for (name, link, want) in cases {
            assert_eq!(taken(&link), want, "{name}");
        }
    }

    #[test]
    fn the_line_names_what_the_bus_said() {
        let said = line("org.quire.Shell is taken");
        assert!(said.contains("org.quire.Shell is taken"));
        assert!(said.contains("this shell stops"));
    }

    #[tokio::test]
    async fn a_name_taken_before_or_after_the_watch_began_ends_it_with_the_line() {
        let held = Intents {
            link: IntentsLink::Unavailable(ClaimError::NameTaken(
                "org.quire.Shell is taken".to_owned(),
            )),
            ..Intents::default()
        };
        // Before: the state already says so.
        let (_tx, rx) = watch::channel(held.clone());
        assert_eq!(
            until_taken(rx).await,
            Some(line("org.quire.Shell is taken"))
        );
        // After: serving first, then the bus says the name is taken.
        let (tx, rx) = watch::channel(Intents {
            link: IntentsLink::Serving,
            ..Intents::default()
        });
        let watching = tokio::spawn(until_taken(rx));
        tx.send_replace(held);
        let said = watching.await.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(said, Some(line("org.quire.Shell is taken")));
        // The service going ends it with nothing to say.
        let (tx, rx) = watch::channel(Intents::default());
        drop(tx);
        assert_eq!(until_taken(rx).await, None);
    }
}
