//! `session.locker`: the setting's words, the lockers sill knows how to run, and the one pure
//! resolution from (setting, kind of session, what is installed) to what the service does.
//!
//! Known lockers, first found wins under `auto` (sill FINDINGS "M7" has the evidence):
//!
//! - **cosmic-greeter**: COSMIC's lock screen is the same binary as its greeter. Run as a user
//!   other than `cosmic-greeter` it is the locker; with logind reachable it stays resident and
//!   waits for logind's `Lock`, and with no system bus it locks at once and exits after a
//!   successful unlock, which is the one-shot shape sill wants. It needs only
//!   `ext-session-lock-v1` from the compositor. It says "session locked" in the journal (its
//!   logs go there, not to stderr) once cosmic-comp has granted the lock.
//! - **swaylock** with `--ready-fd 1`: writes a line to stdout once locked.
//! - **hyprlock**: no ready signal, so it is taken as locked after a short grace.
//!
//! Anything else is a command line run by `/bin/sh -c`, also taken as locked after the grace.
//!
//! `sill` is sill's own lock screen (M11, FINDINGS "M11 freeze" F544): no program, the shell's
//! own `ext-session-lock-v1` surfaces through shell-host, drawn by `sill-surfaces`' lock screen.

use std::path::PathBuf;

use super::machine::{Runner, Triggers};

/// The grace after which a locker with no ready signal is taken as locked.
pub const GRACE_MS: u64 = 1_000;

/// What `session.locker` says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockerChoice {
    /// Pick by session and by what is installed.
    Auto,
    /// Never lock.
    Off,
    /// sill's own lock screen (`sill`).
    Own,
    Known(KnownLocker),
    /// A command line for `/bin/sh -c`.
    Command(String),
}

impl LockerChoice {
    /// The setting's text: `auto` (also empty), `off`, `sill`, a known locker's name, or a
    /// command.
    pub fn parse(text: &str) -> LockerChoice {
        match text.trim() {
            "" | "auto" => LockerChoice::Auto,
            "off" | "none" => LockerChoice::Off,
            "sill" => LockerChoice::Own,
            other => KnownLocker::ALL
                .into_iter()
                .find(|known| known.program() == other)
                .map_or_else(
                    || LockerChoice::Command(other.to_owned()),
                    LockerChoice::Known,
                ),
        }
    }
}

/// A locker sill knows how to run and to hear from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownLocker {
    CosmicGreeter,
    Swaylock,
    Hyprlock,
}

impl KnownLocker {
    /// In `auto`'s order.
    pub const ALL: [KnownLocker; 3] = [
        KnownLocker::CosmicGreeter,
        KnownLocker::Swaylock,
        KnownLocker::Hyprlock,
    ];

    /// The program name, looked up on `PATH`; also the setting's word and the process name.
    pub fn program(self) -> &'static str {
        match self {
            KnownLocker::CosmicGreeter => "cosmic-greeter",
            KnownLocker::Swaylock => "swaylock",
            KnownLocker::Hyprlock => "hyprlock",
        }
    }

    /// How to run the locker found at `program`.
    pub fn plan(self, program: PathBuf) -> LockerPlan {
        let owned = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect()
        };
        let name = Some(self.program().to_owned());
        match self {
            KnownLocker::CosmicGreeter => LockerPlan {
                program,
                args: Vec::new(),
                // No system bus: lock at once, exit after unlocking (instead of waiting,
                // resident, for a logind `Lock` that sill has already acted on). Its log line
                // "session locked" is sill's ready signal.
                env: owned(&[
                    ("DBUS_SYSTEM_BUS_ADDRESS", NO_SYSTEM_BUS),
                    ("RUST_LOG", "warn,cosmic_greeter::locker=info"),
                ]),
                readiness: Readiness::Journal(COSMIC_LOCKED_LINE.to_owned()),
                process_name: name,
            },
            KnownLocker::Swaylock => LockerPlan {
                program,
                args: vec!["--ready-fd".to_owned(), "1".to_owned()],
                env: Vec::new(),
                readiness: Readiness::StdoutLine,
                process_name: name,
            },
            KnownLocker::Hyprlock => LockerPlan {
                program,
                args: Vec::new(),
                env: Vec::new(),
                readiness: Readiness::Grace(GRACE_MS),
                process_name: name,
            },
        }
    }
}

/// A system bus address nothing listens on.
pub const NO_SYSTEM_BUS: &str = "unix:path=/nonexistent/sill-locker-no-system-bus";

/// What cosmic-greeter's locker logs once the compositor granted the lock.
pub const COSMIC_LOCKED_LINE: &str = "session locked";

/// How a started locker says it holds the lock.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Readiness {
    /// Any line on its stdout.
    StdoutLine,
    /// A journal line from its process containing this text.
    Journal(String),
    /// Nothing: taken as locked once it has been running this many milliseconds.
    Grace(u64),
}

/// One locker to start.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockerPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Added to sill's own environment.
    pub env: Vec<(String, String)>,
    pub readiness: Readiness,
    /// The process name (`/proc/<pid>/comm`) of a running one, to adopt instead of starting a
    /// second; `None` for a shell command, which cannot be recognised.
    pub process_name: Option<String>,
}

impl LockerPlan {
    /// `/bin/sh -c command`.
    pub fn shell(command: &str) -> LockerPlan {
        LockerPlan {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_owned(), command.to_owned()],
            env: Vec::new(),
            readiness: Readiness::Grace(GRACE_MS),
            process_name: None,
        }
    }
}

/// What kind of session sill is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Host {
    /// cosmic-session is running for this user: COSMIC's resident locker answers logind.
    CosmicSession,
    /// sill's own session (`dist/sill-session`: `XDG_CURRENT_DESKTOP` names COSMIC, no
    /// cosmic-session).
    SillSession,
    /// Another desktop's session, e.g. sill on the nested compositor under KDE: logind's
    /// signals belong to that desktop's own locker.
    Guest,
}

impl Host {
    /// From whether cosmic-session runs and `XDG_CURRENT_DESKTOP` (colon-separated).
    pub fn of(cosmic_session: SessionManager, current_desktop: &str) -> Host {
        match cosmic_session {
            SessionManager::CosmicSession => Host::CosmicSession,
            SessionManager::Other
                if current_desktop
                    .split(':')
                    .any(|d| d.eq_ignore_ascii_case("COSMIC")) =>
            {
                Host::SillSession
            }
            SessionManager::Other => Host::Guest,
        }
    }

    fn triggers(self) -> Triggers {
        match self {
            Host::SillSession => Triggers::FollowLogind,
            Host::CosmicSession | Host::Guest => Triggers::IgnoreLogind,
        }
    }
}

/// Whether a cosmic-session process runs for this user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionManager {
    CosmicSession,
    Other,
}

/// What the service does, resolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockerMode {
    Off,
    /// Ask logind; COSMIC's resident locker locks.
    Delegate,
    Run {
        plan: LockerPlan,
        triggers: Triggers,
    },
    /// sill's own lock screen: the daemon holds shell-host's session lock while the service's
    /// phase is `Locking` or `Locked` (`Lock::drawer` is `Shell`).
    Own {
        triggers: Triggers,
    },
}

impl LockerMode {
    /// The machine's view of it.
    pub fn runner(&self) -> (Runner, Triggers) {
        match self {
            LockerMode::Off => (Runner::Off, Triggers::IgnoreLogind),
            LockerMode::Delegate => (Runner::Delegates, Triggers::IgnoreLogind),
            LockerMode::Run { triggers, .. } | LockerMode::Own { triggers } => {
                (Runner::Runs, *triggers)
            }
        }
    }
}

/// `choice` in `host`, where `find` says where an installed program is.
pub fn resolve(
    choice: &LockerChoice,
    host: Host,
    find: impl Fn(&str) -> Option<PathBuf>,
) -> LockerMode {
    let run = |plan| LockerMode::Run {
        plan,
        triggers: host.triggers(),
    };
    let found = |known: KnownLocker| find(known.program()).map(|path| known.plan(path));
    match choice {
        LockerChoice::Off => LockerMode::Off,
        LockerChoice::Own => LockerMode::Own {
            triggers: host.triggers(),
        },
        // Under cosmic-session, COSMIC's locker is already resident and answering logind.
        LockerChoice::Auto | LockerChoice::Known(KnownLocker::CosmicGreeter)
            if host == Host::CosmicSession =>
        {
            LockerMode::Delegate
        }
        LockerChoice::Auto => KnownLocker::ALL
            .into_iter()
            .find_map(found)
            .map_or(LockerMode::Off, run),
        LockerChoice::Known(known) => found(*known).map_or(LockerMode::Off, run),
        LockerChoice::Command(command) => run(LockerPlan::shell(command)),
    }
}

#[cfg(test)]
#[path = "choice_tests.rs"]
mod tests;
