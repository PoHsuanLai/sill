//! The model picker a person's first start offers: the assistant has no default model, so the
//! first time the person enters the desktop Settings offers the choice, once.

use serde::{Deserialize, Serialize};

use super::Store;
use crate::launch::SettingsPage;

/// What sill keeps to know the picker was offered: the page it opened. Offered is offered,
/// whether the person picked or skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOffered {
    /// The page that was opened.
    pub page: SettingsPage,
}

/// The page to open at this start: the first-run picker when the record of having offered it
/// was never written, nothing once it was.
pub fn first_model_offer(record: Store) -> Option<SettingsPage> {
    match record {
        Store::Missing => Some(SettingsPage::AssistantModelSetup),
        Store::Present => None,
    }
}
