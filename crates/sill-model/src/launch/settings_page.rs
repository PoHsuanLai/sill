//! The pages of Settings the shell opens by name, and the slug Settings knows each by. This is
//! the one place the slugs are written: a page Settings renames changes here and nowhere else.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// A page of Settings found by name (a settings result the launcher shows): its slug as Settings
/// knows it, and the xdg-activation token that lets Settings take focus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsVisit {
    pub slug: String,
    /// `None` when the compositor refused one.
    pub token: Option<String>,
}

/// A page of Settings the shell opens by `org.freedesktop.Application.ActivateAction("open-page",
/// [slug])`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum SettingsPage {
    /// The picker for the assistant's model: opened when a turn needs a model and none is set.
    #[word(slug = "assistant-model")]
    AssistantModel,
    /// The same picker as the first-run step, with Continue and Skip.
    #[word(slug = "assistant-model-setup")]
    AssistantModelSetup,
}
