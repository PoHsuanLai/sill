//! `SheetView` to quire's `ds-shell::accounts` props: the one place porter's sheet values become
//! what the drawn parts take (PLAN §2.4, design/31 §5.1). It stands in `sill-overlays`, not
//! `sill-model`: the props are quire's (they name `MarkProvider` and the `Word` enums of
//! `ds-shell`), and `sill-model` may reach no design system (`scripts/check-boundary.sh`).
//! Plain values in, plain values out; the draft supplies what the person has typed.
//!
//! | `SheetView`          | drawn as                                             |
//! | -------------------- | ---------------------------------------------------- |
//! | `Consent`            | `ConsentAlert`                                       |
//! | `Providers`          | `ProviderList`                                       |
//! | `SignIn`             | `SignInForm`                                         |
//! | `BrowserWait`        | `BrowserWait`                                        |
//! | `ShowCode`           | `ShowCode`                                           |
//! | `Review`             | `ReviewServices`                                     |
//! | `Working`            | `SignInWorking`                                      |
//! | `Failed`             | `SignInFailed`                                       |
//! | `Done`               | nothing (accountd closes the sheet)                  |

use ds::components::content::provider_mark::MarkProvider;
use ds::prelude::Check;
use ds_shell::accounts::hidden::Hidden;
use ds_shell::accounts::model::{
    AccountChoice, Attempt, ChoiceKey, CopyState, FieldProblem, FieldRole, FieldText, FormField,
    ProblemKind, ProviderEntry, ProviderKey, ProviderPick, Requirement, ServiceKey, ServiceLine,
    ServiceOffer, SignInFault,
};
use porter_core::sheet::{
    Entry, FieldKind, FieldValue, Presence, ProblemKind as PorterProblem, ServiceState, SheetView,
};
use porter_core::{AccountId, ProviderId, Toggle};
use sill_model::accounts::sheet::{CopyMark, Draft, ListKey, OpenSheet};

use super::words::{
    absence_of, app_name, fault_of, limit_of, mark_of, provider_label, request_phrase, service_key,
    service_name,
};

/// What the consent alert is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConsentProps {
    pub app: String,
    pub request: String,
    pub choices: Vec<AccountChoice>,
    pub chosen: Option<ChoiceKey>,
}

/// What the provider list is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ProvidersProps {
    pub providers: Vec<ProviderEntry>,
    pub query: String,
    pub cursor: Option<ProviderPick>,
}

/// What the sign-in form is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SignInProps {
    pub provider: String,
    pub mark: MarkProvider,
    pub fields: Vec<FormField>,
    pub problem: Option<FieldProblem>,
}

/// What the review is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ReviewProps {
    pub account: String,
    pub services: Vec<ServiceLine>,
    pub allow: Option<String>,
}

/// The step on screen, as props.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Step {
    Consent(ConsentProps),
    Providers(ProvidersProps),
    SignIn(SignInProps),
    Browser {
        provider: String,
        url: String,
        copied: CopyState,
    },
    Code {
        provider: String,
        code: String,
        url: String,
        copied: CopyState,
    },
    Review(ReviewProps),
    Working {
        provider: String,
    },
    Failed {
        provider: String,
        why: SignInFault,
    },
}

impl Step {
    /// What the window's title area says: the step's own title, as the step words it.
    pub(super) fn title(&self) -> String {
        match self {
            // The alert's own headline says what the app wants; the window is named for the kind
            // of question.
            Step::Consent(_) => "Account Access".to_owned(),
            Step::Providers(_) => "Add Account".to_owned(),
            Step::SignIn(props) => format!("Sign in to {}", props.provider),
            Step::Browser { .. } => "Continue in your browser".to_owned(),
            Step::Code { .. } => "Enter this code".to_owned(),
            Step::Review(_) => "Choose what to use".to_owned(),
            Step::Working { provider } => format!("Signing in to {provider}"),
            Step::Failed { provider, .. } => format!("Could not add {provider}"),
        }
    }
}

/// The props of `sheet`'s view, or `None` when there is nothing to draw (`Done`).
pub(super) fn step_of(sheet: &OpenSheet) -> Option<Step> {
    let draft = &sheet.draft;
    Some(match &sheet.view {
        SheetView::Consent(ask) => Step::Consent(ConsentProps {
            app: app_name(&ask.app),
            request: request_phrase(ask.kind, ask.class, ask.usage),
            choices: ask
                .accounts
                .iter()
                .map(|choice| AccountChoice {
                    key: ChoiceKey(choice.account.as_str().to_owned()),
                    label: choice.label.0.clone(),
                    provider: mark_of(choice.provider.as_str()),
                })
                .collect(),
            chosen: draft
                .chosen
                .as_ref()
                .map(|account| ChoiceKey(account.as_str().to_owned())),
        }),
        SheetView::Providers(rows) => Step::Providers(ProvidersProps {
            providers: rows
                .iter()
                .map(|row| ProviderEntry {
                    key: ProviderKey(row.id.as_str().to_owned()),
                    label: row.label.clone(),
                    mark: mark_of(&row.mark),
                })
                .collect(),
            query: draft.query.clone(),
            cursor: draft.cursor.as_ref().map(|key| match key {
                ListKey::Provider(id) => {
                    ProviderPick::Provider(ProviderKey(id.as_str().to_owned()))
                }
                ListKey::Other => ProviderPick::Other,
            }),
        }),
        SheetView::SignIn(form) => Step::SignIn(SignInProps {
            provider: provider_label(&form.provider),
            mark: mark_of(form.provider.as_str()),
            fields: draft
                .fields
                .iter()
                .map(|field| FormField {
                    role: role_of(field.kind),
                    requirement: match field.presence {
                        Presence::Required => Requirement::Required,
                        Presence::Optional => Requirement::Optional,
                    },
                    text: match (&field.value, field.entry) {
                        (FieldValue::Secret(text), _) => {
                            FieldText::Secret(Hidden::new(text.expose()))
                        }
                        (FieldValue::Plain(text), Entry::Plain | Entry::Secret) => {
                            FieldText::Plain(text.clone())
                        }
                    },
                })
                .collect(),
            problem: form.problem.map(|problem| FieldProblem {
                role: role_of(problem.field),
                kind: match problem.problem {
                    PorterProblem::Missing => ProblemKind::Missing,
                    PorterProblem::Refused => ProblemKind::Refused,
                },
                attempt: Attempt(draft.submits.0),
            }),
        }),
        SheetView::BrowserWait { provider, url } => Step::Browser {
            provider: provider_label(provider),
            url: url.as_str().to_owned(),
            copied: copy_state(draft),
        },
        SheetView::ShowCode {
            provider,
            user_code,
            url,
        } => Step::Code {
            provider: provider_label(provider),
            code: user_code.0.clone(),
            url: url.as_str().to_owned(),
            copied: copy_state(draft),
        },
        SheetView::Review(review) => Step::Review(ReviewProps {
            account: review.review.label.0.clone(),
            services: review
                .review
                .services
                .iter()
                .map(|row| ServiceLine {
                    key: ServiceKey(service_key(row.kind)),
                    name: service_name(row.kind).to_owned(),
                    offer: match row.state {
                        ServiceState::Offered(shipped) => ServiceOffer::Offered(switch(
                            draft.toggle_of(row.kind).unwrap_or(shipped),
                        )),
                        ServiceState::Absent(reason) => ServiceOffer::Absent(absence_of(reason)),
                    },
                    limit: row.limit.map(limit_of),
                })
                .collect(),
            allow: review.allow.as_ref().map(app_name),
        }),
        SheetView::Working(provider) => Step::Working {
            provider: provider_label(provider),
        },
        SheetView::Failed { provider, fault } => Step::Failed {
            provider: provider_label(provider),
            why: fault_of(*fault),
        },
        SheetView::Done => return None,
    })
}

fn switch(toggle: Toggle) -> Check {
    match toggle {
        Toggle::On => Check::On,
        Toggle::Off => Check::Off,
    }
}

fn copy_state(draft: &Draft) -> CopyState {
    match draft.copied {
        CopyMark::Idle => CopyState::Idle,
        CopyMark::Copied => CopyState::Copied,
    }
}

/// The role a porter field kind plays in the form.
pub(super) fn role_of(kind: FieldKind) -> FieldRole {
    match kind {
        FieldKind::Address => FieldRole::Address,
        FieldKind::Server => FieldRole::Server,
        FieldKind::Username => FieldRole::Username,
        FieldKind::Password => FieldRole::Password,
        FieldKind::ApiKey => FieldRole::ApiKey,
        FieldKind::Token => FieldRole::Token,
    }
}

/// The field kind a form role stands for (the way back).
pub(super) fn kind_of(role: FieldRole) -> FieldKind {
    match role {
        FieldRole::Address => FieldKind::Address,
        FieldRole::Server => FieldKind::Server,
        FieldRole::Username => FieldKind::Username,
        FieldRole::Password => FieldKind::Password,
        FieldRole::ApiKey => FieldKind::ApiKey,
        FieldRole::Token => FieldKind::Token,
    }
}

/// The account a consent key names, when it is one.
pub(super) fn account_of(key: &ChoiceKey) -> Option<AccountId> {
    AccountId::parse(&key.0).ok()
}

/// The provider a list key names, when it is one.
pub(super) fn provider_of(key: &ProviderKey) -> Option<ProviderId> {
    ProviderId::parse(&key.0).ok()
}
