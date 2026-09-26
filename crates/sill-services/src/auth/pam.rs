//! PAM itself, through `nonstick` (MIT; a safe API over libpam, FINDINGS F541). Only
//! `sill auth-check` calls [`check`], in its own short-lived process; the daemon never does.

use std::cell::{Cell, RefCell};
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::rc::Rc;

use nonstick::{
    AuthnFlags, ConversationAdapter, ErrorCode, Result as PamResult, Transaction,
    TransactionBuilder,
};

use super::types::{Attempt, AuthFault, Outcome, PamNote, Verdict};

/// A PAM service name: the file under `/etc/pam.d` that says which modules run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PamService(pub String);

/// sill's own service (`dist/pam.d/sill`).
pub const SILL_SERVICE: &str = "sill";

/// The fallback when sill's file is not installed: every distribution has `login`, and
/// swaylock's own file is only `auth include login`.
pub const FALLBACK_SERVICE: &str = "login";

/// Where PAM looks for a service's file (Linux-PAM: `/etc/pam.d`, then the vendor directories).
pub const SERVICE_DIRS: [&str; 3] = ["/etc/pam.d", "/usr/lib/pam.d", "/usr/etc/pam.d"];

/// Which service to run: [`SILL_SERVICE`] when a file for it exists in any of
/// [`SERVICE_DIRS`] (`exists` answers for a path), else [`FALLBACK_SERVICE`].
pub fn service(exists: impl Fn(&Path) -> bool) -> PamService {
    todo!("M11 fill: a-lock ({})", exists(Path::new(SERVICE_DIRS[0])))
}

/// Run `pam_authenticate` then `pam_acct_mgmt` for `attempt` under `service`, answering the
/// first hidden prompt with the secret and any later prompt with a conversation error.
/// Blocking, and it may take seconds (`pam_unix` delays a failure by about 2 s).
pub fn check(service: &PamService, attempt: &Attempt) -> Verdict {
    let notes = Rc::new(RefCell::new(Vec::new()));
    let conversation = Answers {
        secret: OsString::from(attempt.secret.expose()),
        asked: Cell::new(Asked::Not),
        notes: Rc::clone(&notes),
    };
    let result = TransactionBuilder::new_with_service(&service.0)
        .username(&attempt.user.0)
        .build(conversation.into_conversation())
        .and_then(|mut transaction| {
            transaction.authenticate(AuthnFlags::empty())?;
            transaction.account_management(AuthnFlags::empty())
        });
    Verdict {
        outcome: outcome_of(result),
        notes: notes.take(),
    }
}

/// PAM's answer as an [`Outcome`]: a refusal of the person or the account counts as a failure;
/// PAM or a module failing to run does not.
fn outcome_of(result: PamResult<()>) -> Outcome {
    match result {
        Ok(()) => Outcome::Accepted,
        Err(
            ErrorCode::AuthenticationError
            | ErrorCode::PermissionDenied
            | ErrorCode::UserUnknown
            | ErrorCode::MaxTries
            | ErrorCode::AccountExpired
            | ErrorCode::NewAuthTokRequired
            | ErrorCode::CredentialsInsufficient
            | ErrorCode::ConversationError,
        ) => Outcome::Refused,
        Err(_) => Outcome::Unavailable(AuthFault::ServiceError),
    }
}

/// Whether the secret has been handed over yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    Not,
    Once,
}

/// The conversation: the secret for the first hidden prompt, the modules' messages kept.
struct Answers {
    secret: OsString,
    asked: Cell<Asked>,
    notes: Rc<RefCell<Vec<PamNote>>>,
}

impl ConversationAdapter for Answers {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Err(ErrorCode::ConversationError)
    }

    fn masked_prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        match self.asked.replace(Asked::Once) {
            Asked::Not => Ok(self.secret.clone()),
            Asked::Once => Err(ErrorCode::ConversationError),
        }
    }

    fn error_msg(&self, message: impl AsRef<OsStr>) {
        let text = message.as_ref().to_string_lossy().into_owned();
        self.notes.borrow_mut().push(PamNote::Error(text));
    }

    fn info_msg(&self, message: impl AsRef<OsStr>) {
        let text = message.as_ref().to_string_lossy().into_owned();
        self.notes.borrow_mut().push(PamNote::Info(text));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{PamService, outcome_of, service};
    use crate::auth::types::{AuthFault, Outcome};
    use nonstick::ErrorCode;

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn sills_own_file_wins_else_login() {
        let cases: [(&[&str], &str); 4] = [
            (&["/etc/pam.d/sill"], "sill"),
            (&["/usr/lib/pam.d/sill"], "sill"),
            (&["/etc/pam.d/login"], "login"),
            (&[], "login"),
        ];
        for (present, want) in cases {
            let got = service(|path: &Path| present.iter().any(|p| Path::new(p) == path));
            assert_eq!(got, PamService(want.to_owned()), "{present:?}");
        }
    }

    #[test]
    fn pams_answers_map_onto_outcomes() {
        let cases = [
            (Ok(()), Outcome::Accepted),
            (Err(ErrorCode::AuthenticationError), Outcome::Refused),
            (Err(ErrorCode::MaxTries), Outcome::Refused),
            (
                Err(ErrorCode::ServiceError),
                Outcome::Unavailable(AuthFault::ServiceError),
            ),
            (
                Err(ErrorCode::OpenError),
                Outcome::Unavailable(AuthFault::ServiceError),
            ),
        ];
        for (result, want) in cases {
            assert_eq!(outcome_of(result), want, "{result:?}");
        }
    }
}
