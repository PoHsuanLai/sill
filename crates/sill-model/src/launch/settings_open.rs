//! Opening Settings at a place: the link (the text form of Settings' own target type, which every
//! entrance hands over) and the activation token that lets Settings take focus.

use serde::{Deserialize, Serialize};
use sill_launcher::SettingsLink;

/// A request to open Settings at `link`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsOpen {
    pub link: SettingsLink,
    /// `None` when the compositor refused one.
    pub token: Option<String>,
}

impl SettingsOpen {
    /// A request with no token.
    pub fn at(link: SettingsLink) -> SettingsOpen {
        SettingsOpen { link, token: None }
    }

    /// This request carrying `token`.
    pub fn with_token(self, token: Option<String>) -> SettingsOpen {
        SettingsOpen { token, ..self }
    }
}

/// The picker for the assistant's model: opened when a turn needs a model and none is set.
pub fn assistant_model() -> SettingsLink {
    SettingsLink("page:assistant-model".to_owned())
}

/// The same picker as the first-run step, with Continue and Skip.
pub fn assistant_model_setup() -> SettingsLink {
    SettingsLink("page:assistant-model-setup".to_owned())
}
