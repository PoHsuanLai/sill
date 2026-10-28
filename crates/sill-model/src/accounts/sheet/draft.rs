//! The draft of a sheet: built from the view, and carried across an update that does not change
//! what the person is typing into.

use porter_capability::KindToggle;
use porter_capability::Toggle;
use porter_core::AccountId;
use porter_core::ProviderId;
use porter_core::SecretText;
use porter_sheet::{
    Entry, FieldAnswer, FieldKind, FieldProblem, FieldSpec, FieldValue, Presence, ServiceState,
    SheetView, form_problem, refit,
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
            // A port starts empty: empty is the usual one, which the form shows as a hint.
            (Entry::Plain, _) if matches!(spec.kind, FieldKind::Port | FieldKind::OutgoingPort) => {
                FieldValue::Plain(String::new())
            }
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

impl Draft {
    /// What the form holds as answers: plain text trimmed, secrets as they are.
    pub fn answers(&self) -> Vec<FieldAnswer> {
        self.fields
            .iter()
            .map(|field| FieldAnswer {
                kind: field.kind,
                value: match &field.value {
                    FieldValue::Plain(text) => FieldValue::Plain(text.trim().to_owned()),
                    secret @ FieldValue::Secret(_) => secret.clone(),
                },
            })
            .collect()
    }

    /// What is wrong with the form as typed, if anything (porter's `form_problem` over the form
    /// the view asked for, reshaped by the answers). The form stays unsent while this is `Some`.
    pub fn problem(&self, specs: &[FieldSpec]) -> Option<FieldProblem> {
        let answers = self.answers();
        form_problem(&refit(specs, &answers), &answers)
    }

    /// The draft with the form reshaped to its answers: the fields porter's `refit` of `specs`
    /// gives, each keeping what was typed. A typed host follows the protocol (porter retargets a
    /// guessed `imap.` or `pop.` name); a port keeps only what was typed, since empty means the
    /// usual one; a secret does not survive, as everywhere.
    pub(super) fn refitted(self, specs: &[FieldSpec]) -> Draft {
        let shaped = refit(specs, &self.answers());
        let fields = shaped
            .iter()
            .map(|spec| {
                let kept = self.fields.iter().find(|field| field.kind == spec.kind);
                let mut field = FieldDraft::of(spec, kept);
                match (spec.kind, kept) {
                    (FieldKind::Server, Some(_)) if spec.prefill.is_some() => {
                        field.value = FieldValue::Plain(spec.prefill.clone().unwrap_or_default());
                    }
                    _ => {}
                }
                field
            })
            .collect();
        Draft { fields, ..self }
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
        }
        // The form comes back in the shape it was first sent in; the person's protocol decides.
        .refitted(&now.fields),
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
                    ServiceState::Offered(toggle) => Some(KindToggle {
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
    choices: &[KindToggle],
    kind: porter_capability::CapabilityKind,
    toggle: Toggle,
) -> Vec<KindToggle> {
    choices
        .iter()
        .map(|choice| match choice.kind == kind {
            true => KindToggle { kind, toggle },
            false => *choice,
        })
        .collect()
}
