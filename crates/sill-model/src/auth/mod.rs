//! The lock screen's password check: the account, the attempts, the gate that decides, and the
//! helper's wire.

mod command;
pub mod gate;
mod model;
pub mod protocol;
mod types;

pub use command::{AuthCommand, FakeVerdict};
pub use gate::{Checks, GateIn, GateOut};
pub use model::{Auth, AuthPhase};
pub use types::{
    Account, Attempt, Attempts, AuthFault, AvatarSource, Outcome, PamNote, PamService, RealName,
    Secret, UserName, Verdict,
};
