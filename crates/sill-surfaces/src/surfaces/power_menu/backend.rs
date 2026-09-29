//! Where the power menu's chosen action goes, behind one trait: the session service, or a dry
//! run of the menu's own that offers every action and only records the choice (tests). The
//! daemon's menus always use the session service, whose own session actions may be a dry run
//! (`sill run --session-dry-run`); the live backend reports that as its kind, and the acceptance
//! script proves it before it presses Enter. Nothing here ever runs on its own: only a chosen
//! button calls [`SessionBackend::run`].

use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sill_model::session::{Session, SessionAction, SessionCommand};
use sill_services::env::ActionReach;
use sill_services::{Control, ServiceError, Services};
use tokio::sync::watch;

use crate::surfaces::power_menu::labels::action_label;

/// Which backend a surface runs on, for its log line (`power_menu.backend=<name>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Live,
    DryRun,
}

impl BackendKind {
    /// The log word.
    pub fn name(self) -> &'static str {
        match self {
            BackendKind::Live => "live",
            BackendKind::DryRun => "dry-run",
        }
    }
}

/// What the power menu reads its offers from and sends its chosen action to.
pub trait SessionBackend: Send + Sync {
    /// Which backend this is.
    fn kind(&self) -> BackendKind;
    /// logind's offers, followed.
    fn offers(&self) -> watch::Receiver<Session>;
    /// Ask for fresh offers (the menu is opening).
    fn refresh(&self);
    /// Carry out `action`.
    fn run(&self, action: SessionAction) -> Result<(), ServiceError>;
}

/// The backend as a root context. Clone is cheap.
#[derive(Clone)]
pub struct PowerMenuBackend(pub Arc<dyn SessionBackend>);

impl fmt::Debug for PowerMenuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PowerMenuBackend")
            .field(&self.0.kind())
            .finish()
    }
}

impl PowerMenuBackend {
    /// The session service in `services`.
    pub fn live(services: &Services) -> PowerMenuBackend {
        PowerMenuBackend(Arc::new(LiveSession {
            offers: services.session.state.clone(),
            control: services.session.control.clone(),
            kind: match services.session_actions.reach() {
                ActionReach::Logind => BackendKind::Live,
                ActionReach::DryRun => BackendKind::DryRun,
            },
        }))
    }

    /// A dry run (see [`DryRunSession`]).
    pub fn dry_run(session: DryRunSession) -> PowerMenuBackend {
        PowerMenuBackend(Arc::new(session))
    }
}

/// The session service: `SessionCommand::Run` and `Refresh`. Its kind is the service's own:
/// a daemon whose session actions are a dry run records every choice there.
struct LiveSession {
    offers: watch::Receiver<Session>,
    control: Control<SessionCommand>,
    kind: BackendKind,
}

impl SessionBackend for LiveSession {
    fn kind(&self) -> BackendKind {
        self.kind
    }

    fn offers(&self) -> watch::Receiver<Session> {
        self.offers.clone()
    }

    fn refresh(&self) {
        let _gone_is_inert = self.control.send(SessionCommand::Refresh);
    }

    fn run(&self, action: SessionAction) -> Result<(), ServiceError> {
        self.control.send(SessionCommand::Run(action))
    }
}

/// Every action offered, none carried out: each chosen action is kept in memory, printed as
/// `power_menu.dry_run would=<Label>` and, with a record file, appended to it as a line.
#[derive(Clone)]
pub struct DryRunSession {
    offers: watch::Receiver<Session>,
    ran: Arc<Mutex<Vec<SessionAction>>>,
    record: Option<PathBuf>,
}

impl fmt::Debug for DryRunSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DryRunSession")
            .field("record", &self.record)
            .finish_non_exhaustive()
    }
}

impl DryRunSession {
    /// A dry run offering `session`, recording to `record` when given.
    pub fn new(session: Session, record: Option<PathBuf>) -> DryRunSession {
        let (tx, offers) = watch::channel(session);
        // The value never changes; the receiver keeps reading it after the sender is gone.
        drop(tx);
        DryRunSession {
            offers,
            ran: Arc::default(),
            record,
        }
    }

    /// A dry run offering whatever `offers` holds (the session service's, read only: logind's
    /// answers), recording to `record` when given. It holds no way to reach logind's actions.
    pub fn following(offers: watch::Receiver<Session>, record: Option<PathBuf>) -> DryRunSession {
        DryRunSession {
            offers,
            ran: Arc::default(),
            record,
        }
    }

    /// The actions chosen so far, oldest first.
    pub fn ran(&self) -> Vec<SessionAction> {
        self.ran.lock().map(|ran| ran.clone()).unwrap_or_default()
    }
}

/// The dry run's line for `action`.
pub fn dry_run_line(action: SessionAction) -> String {
    format!("power_menu.dry_run would={}", action_label(action))
}

impl SessionBackend for DryRunSession {
    fn kind(&self) -> BackendKind {
        BackendKind::DryRun
    }

    fn offers(&self) -> watch::Receiver<Session> {
        self.offers.clone()
    }

    fn refresh(&self) {}

    fn run(&self, action: SessionAction) -> Result<(), ServiceError> {
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
                eprintln!("sill: power menu: recording to {}: {error}", path.display());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendKind, DryRunSession, SessionBackend, dry_run_line};
    use sill_model::session::Session;
    use sill_model::session::SessionAction;

    #[test]
    fn a_dry_run_offers_everything_and_only_records() {
        let dir = std::env::temp_dir().join(format!("sill-dry-run-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let record = dir.join("record");
        let _ = std::fs::remove_file(&record);
        let session = DryRunSession::new(Session::everything(), Some(record.clone()));
        assert_eq!(session.kind(), BackendKind::DryRun);
        assert_eq!(*session.offers().borrow(), Session::everything());
        assert!(session.run(SessionAction::Restart).is_ok());
        assert!(session.run(SessionAction::PowerOff).is_ok());
        assert_eq!(
            session.ran(),
            vec![SessionAction::Restart, SessionAction::PowerOff]
        );
        let written = std::fs::read_to_string(&record).unwrap_or_default();
        assert_eq!(
            written,
            "power_menu.dry_run would=Restart\npower_menu.dry_run would=Shut Down\n"
        );
        assert_eq!(
            dry_run_line(SessionAction::Suspend),
            "power_menu.dry_run would=Suspend"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
