//! The draft of a sheet: built from the view, and carried across an update that does not change
//! what the person is typing into.

use porter_core::AccountId;
use porter_core::ProviderId;
use porter_core::SecretText;
use porter_core::Toggle;
use porter_core::sheet::{
    Entry, FieldKind, FieldSpec, FieldValue, Presence, ServiceChoice, ServiceState, SheetView,
};

use super::model::Draft;

/// A row of the provider list: a provider, or the generic "Other" row quire's list adds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListKey {
    /// A provider of the list.
    Provider(ProviderId),
    /// "Other…": any server that speaks the open protocols.
    Other,
}

/// How many times a form has been sent; a refused field shakes once for each new count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SubmitCount(pub u32);

/// Whether the link or code on screen has been copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopyMark {
    /// Nothing copied lately.
    #[default]
    Idle,
    /// The shell just copied it.
    Copied,
}

/// One field of the form, with what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDraft {
    /// What it is for.
    pub kind: FieldKind,
    /// Shown or hidden.
    pub entry: Entry,
    /// Required or optional.
    pub presence: Presence,
    /// What it holds: the variant follows `entry`.
    pub value: FieldValue,
}

impl FieldDraft {
    fn of(spec: &FieldSpec, kept: Option<&FieldDraft>) -> FieldDraft {
        let value = match (spec.entry, kept.map(|field| &field.value)) {
            (Entry::Plain, Some(FieldValue::Plain(text))) => FieldValue::Plain(text.clone()),
            (Entry::Plain, _) => FieldValue::Plain(spec.prefill.clone().unwrap_or_default()),
            // A secret never survives a new view: what was sent is gone, and so is the rest.
            (Entry::Secret, _) => FieldValue::Secret(SecretText::new("")),
        };
        FieldDraft {
            kind: spec.kind,
            entry: spec.entry,
            presence: spec.presence,
            value,
        }
    }

    /// Whether it holds nothing but blanks.
    pub fn is_blank(&self) -> bool {
        match &self.value {
            FieldValue::Plain(text) => text.trim().is_empty(),
            FieldValue::Secret(text) => text.expose().is_empty(),
        }
    }
}

/// The draft for `view`, from the draft `old` had for the view before it. The same view again
/// keeps everything; a new view of the same step (the form again after a refusal) keeps what a
/// person would not retype; any other view starts clean.
pub(super) fn drafted(old: &Draft, before: &SheetView, view: &SheetView) -> Draft {
    if before == view {
        return old.clone();
    }
    match (before, view) {
        (SheetView::Providers(_), SheetView::Providers(rows)) => Draft {
            cursor: old.cursor.clone().filter(|key| {
                rows.iter()
                    .any(|row| key == &ListKey::Provider(row.id.clone()))
            }),
            query: old.query.clone(),
            ..Draft::default()
        },
        (SheetView::SignIn(was), SheetView::SignIn(now)) if was.provider == now.provider => Draft {
            fields: fields_of(&now.fields, &old.fields),
            submits: old.submits,
            ..Draft::default()
        },
        (SheetView::Consent(_), SheetView::Consent(_)) => Draft {
            chosen: old.chosen.clone(),
            ..Draft::default()
        },
        _ => fresh(view),
    }
}

/// The draft of a view that follows a different step, or the first.
pub(super) fn fresh(view: &SheetView) -> Draft {
    match view {
        SheetView::SignIn(form) => Draft {
            fields: fields_of(&form.fields, &[]),
            ..Draft::default()
        },
        SheetView::Review(review) => Draft {
            choices: review
                .review
                .services
                .iter()
                .filter_map(|row| match row.state {
                    ServiceState::Offered(toggle) => Some(ServiceChoice {
                        kind: row.kind,
                        toggle,
                    }),
                    ServiceState::Absent(_) => None,
                })
                .collect(),
            ..Draft::default()
        },
        _ => Draft::default(),
    }
}

fn fields_of(specs: &[FieldSpec], kept: &[FieldDraft]) -> Vec<FieldDraft> {
    specs
        .iter()
        .map(|spec| FieldDraft::of(spec, kept.iter().find(|field| field.kind == spec.kind)))
        .collect()
}

/// `draft` with its first consent answer picked: nothing to carry, the account is the first until
/// the person chooses.
pub(super) fn chosen_among(draft: &Draft, accounts: &[AccountId]) -> Option<AccountId> {
    draft
        .chosen
        .clone()
        .filter(|account| accounts.contains(account))
        .or_else(|| accounts.first().cloned())
}

/// `choices` with `kind` set to `toggle`, when the provider offers that kind.
pub(super) fn toggled(
    choices: &[ServiceChoice],
    kind: porter_core::CapabilityKind,
    toggle: Toggle,
) -> Vec<ServiceChoice> {
    choices
        .iter()
        .map(|choice| match choice.kind == kind {
            true => ServiceChoice { kind, toggle },
            false => *choice,
        })
        .collect()
}
