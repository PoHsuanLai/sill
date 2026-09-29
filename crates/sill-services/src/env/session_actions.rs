//! Where the session's actions (Lock, Log Out, Restart, Shut Down, Suspend) go: to logind, or,
//! in a dry run, only into a record. The session service asks this before it ever calls logind,
//! so a dry run cannot reach it from the power menu, the launcher's rows or anywhere else.

use std::fmt;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::session::{Session, SessionAction};

/// Whether chosen actions reach logind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionReach {
    /// logind carries them out.
    Logind,
    /// They are only recorded.
    DryRun,
}

/// The session's actions: what is offered and where a chosen one goes.
pub trait SessionActions: Send + Sync + fmt::Debug {
    /// Whether chosen actions reach logind (for the surfaces' log lines).
    fn reach(&self) -> ActionReach;
    /// What the session offers, given logind's answers (`Session::default()` without logind).
    fn offers(&self, logind: Session) -> Session;
    /// Take `action`: [`ActionReach::Logind`] when logind is to carry it out,
    /// [`ActionReach::DryRun`] when it was only recorded.
    fn act(&self, action: SessionAction) -> ActionReach;
}

/// logind's offers and logind's actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogindActions;

impl SessionActions for LogindActions {
    fn reach(&self) -> ActionReach {
        ActionReach::Logind
    }

    fn offers(&self, logind: Session) -> Session {
        logind
    }

    fn act(&self, _action: SessionAction) -> ActionReach {
        ActionReach::Logind
    }
}

/// What a dry run offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DryRunOffers {
    /// Every action.
    #[default]
    Everything,
    /// What logind offers (its answers are read, never its actions), so an action this machine
    /// refuses is drawn disabled.
    Logind,
}

/// No action carried out: each chosen one is kept in memory, printed as
/// `session.dry_run would=<Action>` and, with a record file, appended to it as a line.
pub struct DryRunActions {
    offers: DryRunOffers,
    record: Option<PathBuf>,
    ran: Mutex<Vec<SessionAction>>,
}

impl fmt::Debug for DryRunActions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DryRunActions")
            .field("offers", &self.offers)
            .field("record", &self.record)
            .finish_non_exhaustive()
    }
}

impl DryRunActions {
    /// A dry run offering `offers`, recording to `record` when given.
    pub fn new(offers: DryRunOffers, record: Option<PathBuf>) -> DryRunActions {
        DryRunActions {
            offers,
            record,
            ran: Mutex::new(Vec::new()),
        }
    }

    /// The actions chosen so far, oldest first.
    pub fn ran(&self) -> Vec<SessionAction> {
        self.ran.lock().map(|ran| ran.clone()).unwrap_or_default()
    }
}

/// The dry run's line for `action`.
pub fn dry_run_line(action: SessionAction) -> String {
    format!("session.dry_run would={action:?}")
}

impl SessionActions for DryRunActions {
    fn reach(&self) -> ActionReach {
        ActionReach::DryRun
    }

    fn offers(&self, logind: Session) -> Session {
        match self.offers {
            DryRunOffers::Everything => Session::everything(),
            DryRunOffers::Logind => logind,
        }
    }

    fn act(&self, action: SessionAction) -> ActionReach {
        if let Ok(mut ran) = self.ran.lock() {
            ran.push(action);
        }
        let line = dry_run_line(action);
        eprintln!("{line}");
        if let Some(path) = &self.record {
            let appended = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "{line}"));
            if let Err(error) = appended {
                eprintln!("sill: session: recording to {}: {error}", path.display());
            }
        }
        ActionReach::DryRun
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionReach, DryRunActions, DryRunOffers, LogindActions, SessionActions, dry_run_line,
    };
    use sill_testkit::TempDir;
    use crate::session::{Session, SessionAction};

    #[test]
    fn a_dry_run_offers_as_told_and_only_records() {
        let scratch = TempDir::new("dry-run");
        let record = crate::env::Env::isolated(scratch.path()).dirs.state.join("record");
        std::fs::create_dir_all(&crate::env::Env::isolated(scratch.path()).dirs.state).unwrap_or_else(|e| panic!("{e}"));
        let dry = DryRunActions::new(DryRunOffers::Everything, Some(record.clone()));
        assert_eq!(dry.offers(Session::default()), Session::everything());
        assert_eq!(dry.act(SessionAction::Restart), ActionReach::DryRun);
        assert_eq!(dry.act(SessionAction::PowerOff), ActionReach::DryRun);
        assert_eq!(
            dry.ran(),
            vec![SessionAction::Restart, SessionAction::PowerOff]
        );
        assert_eq!(
            std::fs::read_to_string(&record).unwrap_or_default(),
            "session.dry_run would=Restart\nsession.dry_run would=PowerOff\n"
        );
        let following = DryRunActions::new(DryRunOffers::Logind, None);
        assert_eq!(following.offers(Session::default()), Session::default());
        assert_eq!(
            dry_run_line(SessionAction::Suspend),
            "session.dry_run would=Suspend"
        );
    }

    #[test]
    fn logind_offers_and_acts_as_itself() {
        assert_eq!(
            LogindActions.offers(Session::everything()),
            Session::everything()
        );
        assert_eq!(
            LogindActions.act(SessionAction::Suspend),
            ActionReach::Logind
        );
        assert_eq!(LogindActions.reach(), ActionReach::Logind);
    }
}
