//! What the polkit prompt follows.

use serde::{Deserialize, Serialize};

use super::{PromptView, Registration};

/// What the polkit prompt follows (`Services.polkit_agent`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Polkit {
    pub registration: Registration,
    /// The request on screen; the others wait in order.
    pub current: Option<PromptView>,
    pub waiting: u32,
}
