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
    AccountChoice, Attempt, Choice, ChoiceKey, CopyState, FieldProblem, FieldRole, FieldText,
    FormField, FormPart, ProblemKind, ProviderEntry, ProviderKey, ProviderPick, Requirement,
    ServiceKey, ServiceLine, ServiceOffer, SignInFault,
};
use ds_shell::prelude::WorkingFor;
use porter_core::sheet::{
    Entry, FieldKind, FieldValue, Presence, ProblemKind as PorterProblem, RowKind, ServiceState,
    SheetView,
};
use porter_core::{AccountId, LauncherSession, ProviderId, Toggle};
use sill_model::accounts::sheet::{CopyMark, Draft, ListKey, OpenSheet};

use super::words::{
    absence_of, app_name, choice_label, fault_of, limit_of, mark_of, provider_name, request_phrase,
    service_key, usual_port, waiting_on,
};

/// What the consent alert is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConsentProps {
    pub app: String,
    pub request: String,
    pub choices: Vec<AccountChoice>,
    pub chosen: Option<ChoiceKey>,
    /// The launcher session the ask was made inside: when present, "This Session Only" is offered.
    pub session: Option<LauncherSession>,
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
        waiting_for: WorkingFor,
    },
    Failed {
        provider: String,
        why: SignInFault,
        mark: Option<MarkProvider>,
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
            Step::Working { provider, .. } => format!("Signing in to {provider}"),
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
            session: ask.session.clone(),
        }),
        SheetView::Providers(rows) => Step::Providers(ProvidersProps {
            // The generic row is ProviderList's own "Other…", so it is not listed twice.
            providers: rows
                .iter()
                .filter(|row| row.kind != RowKind::Generic)
                .map(|row| {
                    ProviderEntry::new(
                        ProviderKey(row.id.as_str().to_owned()),
                        row.label.clone(),
                        mark_of(&row.mark),
                    )
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
            provider: provider_name(&form.provider, form.row.as_ref()),
            mark: mark_of(form.provider.as_str()),
            fields: form_fields(draft),
            // A refusal from accountd first; else what cannot be right in what is typed (a
            // port that is no number), marked on its field as it is typed. An empty required
            // field is not marked: Continue says it.
            problem: form
                .problem
                .map(|problem| (problem.field, problem.problem))
                .or_else(|| {
                    draft
                        .problem(&form.fields)
                        .filter(|problem| problem.problem == PorterProblem::Invalid)
                        .map(|problem| (problem.field, problem.problem))
                })
                .map(|(field, problem)| FieldProblem {
                    role: role_of(field),
                    kind: match problem {
                        PorterProblem::Missing => ProblemKind::Missing,
                        PorterProblem::Refused => ProblemKind::Refused,
                        PorterProblem::Invalid => ProblemKind::Invalid,
                    },
                    attempt: Attempt(draft.submits.0),
                }),
        }),
        SheetView::BrowserWait { provider, row, url } => Step::Browser {
            provider: provider_name(provider, row.as_ref()),
            url: url.as_str().to_owned(),
            copied: copy_state(draft),
        },
        SheetView::ShowCode {
            provider,
            row,
            user_code,
            url,
        } => Step::Code {
            provider: provider_name(provider, row.as_ref()),
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
                    name: row.kind.display_name().to_owned(),
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
        SheetView::Working { provider, row } => Step::Working {
            provider: provider_name(provider, row.as_ref()),
            waiting_for: waiting_on(row.as_ref()),
        },
        SheetView::Failed {
            provider,
            row,
            fault,
        } => Step::Failed {
            provider: provider_name(provider, row.as_ref()),
            why: fault_of(*fault),
            mark: Some(mark_of(provider.as_str())),
        },
        SheetView::Done => return None,
    })
}

/// The form's fields as drawn: what each holds, its choices, its hint and its group.
fn form_fields(draft: &Draft) -> Vec<FormField> {
    let held = |kind| {
        draft
            .fields
            .iter()
            .find(|field| field.kind == kind)
            .and_then(|field| match &field.value {
                FieldValue::Plain(text) => Some(text.as_str()),
                FieldValue::Secret(_) => None,
            })
    };
    let protocol = held(FieldKind::Protocol).unwrap_or("imap");
    // Only the hand-typed server form is grouped; the others stay one plain list.
    let manual = draft.fields.iter().any(|f| f.kind == FieldKind::Protocol);
    let security = |kind| held(kind).unwrap_or("tls");
    draft
        .fields
        .iter()
        .map(|field| {
            let base = FormField::new(
                role_of(field.kind),
                match field.presence {
                    Presence::Required => Requirement::Required,
                    Presence::Optional => Requirement::Optional,
                },
                match (&field.value, field.entry) {
                    (FieldValue::Secret(text), _) => FieldText::Secret(Hidden::new(text.expose())),
                    (FieldValue::Plain(text), Entry::Plain | Entry::Secret) => {
                        FieldText::Plain(text.clone())
                    }
                },
            );
            let base = match field.kind.choices() {
                [] => base,
                slugs => base.choosing(
                    slugs
                        .iter()
                        .map(|slug| Choice::new(*slug, choice_label(slug)))
                        .collect(),
                ),
            };
            let base = match field.kind {
                FieldKind::Port => base
                    .hinted(usual_port(protocol, false, security(FieldKind::Security)).to_string()),
                FieldKind::OutgoingPort => base.hinted(
                    usual_port(protocol, true, security(FieldKind::OutgoingSecurity)).to_string(),
                ),
                _ => base,
            };
            match part_of(field.kind).filter(|_| manual) {
                Some(part) => base.in_part(part),
                None => base,
            }
        })
        .collect()
}

/// The group a field of the hand-typed server form sits in; the other forms have none.
fn part_of(kind: FieldKind) -> Option<FormPart> {
    match kind {
        FieldKind::Protocol
        | FieldKind::Server
        | FieldKind::Port
        | FieldKind::Security
        | FieldKind::SessionUrl => Some(FormPart::Incoming),
        FieldKind::OutgoingServer | FieldKind::OutgoingPort | FieldKind::OutgoingSecurity => {
            Some(FormPart::Outgoing)
        }
        FieldKind::Username | FieldKind::Password | FieldKind::Token => Some(FormPart::SignIn),
        FieldKind::Address | FieldKind::ApiKey => None,
    }
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
        FieldKind::Protocol => FieldRole::Protocol,
        FieldKind::Port => FieldRole::Port,
        FieldKind::Security => FieldRole::Security,
        FieldKind::OutgoingServer => FieldRole::OutgoingServer,
        FieldKind::OutgoingPort => FieldRole::OutgoingPort,
        FieldKind::OutgoingSecurity => FieldRole::OutgoingSecurity,
        FieldKind::SessionUrl => FieldRole::SessionUrl,
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
        FieldRole::Protocol => FieldKind::Protocol,
        FieldRole::Port => FieldKind::Port,
        FieldRole::Security => FieldKind::Security,
        FieldRole::OutgoingServer => FieldKind::OutgoingServer,
        FieldRole::OutgoingPort => FieldKind::OutgoingPort,
        FieldRole::OutgoingSecurity => FieldKind::OutgoingSecurity,
        FieldRole::SessionUrl => FieldKind::SessionUrl,
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
