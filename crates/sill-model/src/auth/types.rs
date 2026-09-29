//! The auth service's words: who is asked, what they typed, what PAM said, what is published.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A login name (`pohsuanlai`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserName(pub String);

/// The name a person goes by (GECOS's first field, or AccountsService's `RealName`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RealName(pub String);

/// What was typed into a password field. Its `Debug` never shows it, it is never serialised,
/// and its bytes are overwritten when it is dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(text: impl Into<String>) -> Secret {
        Secret(text.into())
    }

    /// The text, for the one place that hands it on (the helper's stdin, PAM's conversation).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(..)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        let mut bytes = std::mem::take(&mut self.0).into_bytes();
        bytes.iter_mut().for_each(|byte| *byte = 0);
        std::hint::black_box(&bytes);
    }
}

/// One password check: whose, and what was typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub user: UserName,
    pub secret: Secret,
}

/// How one check ended.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Outcome {
    /// `pam_authenticate` and `pam_acct_mgmt` both succeeded.
    Accepted,
    /// The secret was wrong (or the account refused): counts as a failure.
    Refused,
    /// Nothing could be checked: not the person's fault, never counted.
    Unavailable(AuthFault),
}

/// Why nothing could be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthFault {
    /// The helper could not be started.
    NoHelper,
    /// The helper ended without a verdict, or said something that is not the protocol.
    HelperFailed,
    /// PAM could not start the service (no such file, a module missing).
    ServiceError,
    /// The check ran past its time and its helper was killed.
    TimedOut,
}

/// A message a PAM module showed during the check (`PAM_TEXT_INFO`, `PAM_ERROR_MSG`):
/// faillock's "account locked", a fingerprint reader's "place your finger".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PamNote {
    Info(String),
    Error(String),
}

/// What an [`crate::env::Authenticator`] answers for one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Verdict {
    pub outcome: Outcome,
    pub notes: Vec<PamNote>,
}

impl Verdict {
    /// An outcome with no notes.
    pub fn plain(outcome: Outcome) -> Verdict {
        Verdict {
            outcome,
            notes: Vec::new(),
        }
    }
}

/// Where the lock screen's picture of the person comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AvatarSource {
    /// An image file: AccountsService's `IconFile`, else `$HOME/.face`.
    File(PathBuf),
    /// No picture: quire's `Avatar` draws the initials on the person colour.
    #[default]
    Initials,
}

/// Who the lock screen asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Account {
    pub user: UserName,
    pub name: RealName,
    pub avatar: AvatarSource,
}

/// Failed attempts since the last success.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Attempts(pub u32);

/// A PAM service name: the file under `/etc/pam.d` that says which modules run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PamService(pub String);
