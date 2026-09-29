//! The polkit agent: requests, identities, the prompt and the machine that answers polkitd.

mod command;
pub mod helper;
pub mod identity;
pub mod machine;
mod model;
mod types;

pub use command::{FakeRequest, PolkitCommand};
pub use helper::{HelperLine, HelperOpening, HelperRoute, opening, parse_helper_line};
pub use identity::pick_identity;
pub use machine::{Completion, Origin, PolkitEffect, PolkitEvent, PolkitState, polkit_of, step};
pub use model::Polkit;
pub use types::{
    ActionId, AgentPolicy, AuthRequest, Cookie, Echo, FAKE_SECRET, Gid, Identity, LastTry,
    PromptPhase, PromptView, Registration, Uid,
};
