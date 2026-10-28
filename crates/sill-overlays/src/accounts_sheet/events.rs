//! What the drawn parts report, as what the person did on the sheet (`SheetAction`): the way back
//! of `props.rs`. A key the shell cannot parse (an account or provider that is not one) is no
//! action.

use ds::prelude::Check;
use ds_shell::accounts::model::{
    AllowScope, ChoiceKey, ConsentAnswer, FieldRole, FieldText, ProviderPick, ServiceKey,
};
use porter_capability::Toggle;
use porter_core::consent::GrantScope;
use porter_core::{LauncherSession, SecretText};
use porter_sheet::FieldValue;
use sill_model::accounts::sheet::{ListKey, SheetAction};

use super::props::{account_of, kind_of, provider_of};
use super::words::service_of;

/// The consent alert's answer; `session` is the launcher session the ask was made inside, which
/// "This Session Only" grants.
pub(super) fn consent(
    answer: ConsentAnswer,
    session: Option<&LauncherSession>,
) -> Option<SheetAction> {
    match answer {
        ConsentAnswer::Allow { scope, .. } => Some(SheetAction::Allow(match scope {
            AllowScope::Once => GrantScope::Once,
            AllowScope::Always => GrantScope::Always,
            // The alert offers it only with a session, so there is always one here.
            AllowScope::Session => {
                return session
                    .cloned()
                    .map(|s| SheetAction::Allow(GrantScope::Session(s)));
            }
        })),
        ConsentAnswer::Deny => Some(SheetAction::Deny),
        ConsentAnswer::AddAccount => Some(SheetAction::AddAccount),
        // Escape closes the alert and stores nothing.
        ConsentAnswer::Dismiss => Some(SheetAction::Cancel),
    }
}

/// The pop-up chose an account.
pub(super) fn chosen(key: &ChoiceKey) -> Option<SheetAction> {
    account_of(key).map(SheetAction::Choose)
}

/// A row of the provider list.
pub(super) fn list_key(pick: &ProviderPick) -> Option<ListKey> {
    match pick {
        ProviderPick::Provider(key) => provider_of(key).map(ListKey::Provider),
        ProviderPick::Other => Some(ListKey::Other),
    }
}

/// A form field changed.
pub(super) fn typed(role: FieldRole, text: FieldText) -> SheetAction {
    let value = match text {
        FieldText::Plain(text) => FieldValue::Plain(text),
        FieldText::Secret(text) => FieldValue::Secret(SecretText::new(text.reveal())),
    };
    SheetAction::Type(kind_of(role), value)
}

/// A review switch changed.
pub(super) fn toggled(key: &ServiceKey, check: Check) -> Option<SheetAction> {
    service_of(&key.0).map(|kind| {
        SheetAction::Toggle(
            kind,
            match check {
                Check::On => Toggle::On,
                Check::Off | Check::Mixed => Toggle::Off,
            },
        )
    })
}
