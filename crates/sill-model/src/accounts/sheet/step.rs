//! The pure transition: what the sheets hold after an event, and what accountd must be told.

use porter_core::AccountId;
use porter_core::consent::{ConsentAnswer, GrantScope};
use porter_ids::WebUrl;
use porter_sheet::RowKind;
use porter_sheet::{FieldKind, FieldSpec, FieldValue, Presence, SheetInput, SheetView};

use super::command::{SheetAction, SheetEvent, SheetOut};
use super::draft::{CopyMark, ListKey, SubmitCount, chosen_among, drafted, fresh, toggled};
use super::model::{AccountsSheets, Awaiting, Draft, OpenSheet};

/// `sheets` after `event`, and what it asks of the shell.
pub fn step(sheets: AccountsSheets, event: SheetEvent) -> (AccountsSheets, Vec<SheetOut>) {
    let AccountsSheets { link, mut open } = sheets;
    let mut out = Vec::new();
    let link = match event {
        SheetEvent::Link(next) => next,
        SheetEvent::Opened(handle, view) => {
            if !open.iter().any(|sheet| sheet.handle == handle) {
                out.extend(page_of(&view).map(SheetOut::OpenPage));
                open.push(OpenSheet {
                    handle,
                    draft: fresh(&view),
                    view,
                    awaiting: Awaiting::Nothing,
                });
            }
            link
        }
        SheetEvent::Updated(handle, view) => {
            if let Some(sheet) = open.iter_mut().find(|sheet| sheet.handle == handle) {
                out.extend(page_of(&view).map(SheetOut::OpenPage));
                sheet.draft = drafted(&sheet.draft, &sheet.view, &view);
                sheet.view = view;
                sheet.awaiting = Awaiting::Nothing;
            }
            link
        }
        SheetEvent::Closed(handle) => {
            open.retain(|sheet| sheet.handle != handle);
            link
        }
        SheetEvent::Acted(handle, action) => {
            if let Some(sheet) = open.iter_mut().find(|sheet| sheet.handle == handle) {
                let (draft, input) = act(&sheet.view, std::mem::take(&mut sheet.draft), action);
                sheet.draft = draft;
                // One answer per view: the second of a double press is dropped. A dismissal
                // always goes.
                let input = input.filter(|input| {
                    matches!(input, SheetInput::Dismiss) || sheet.awaiting == Awaiting::Nothing
                });
                if input
                    .as_ref()
                    .is_some_and(|input| !matches!(input, SheetInput::Dismiss))
                {
                    sheet.awaiting = Awaiting::View;
                }
                out.extend(input.map(|input| SheetOut::Reply(handle, input)));
            }
            link
        }
        SheetEvent::Gone => {
            open.clear();
            link
        }
    };
    (AccountsSheets { link, open }, out)
}

/// The provider file "Other..." leads to: the hand-typed mail server form.
const OTHER_PROVIDER: &str = "generic-imap";

/// The page a browser step shows.
fn page_of(view: &SheetView) -> Option<WebUrl> {
    match view {
        SheetView::BrowserWait { url, .. } => Some(url.clone()),
        _ => None,
    }
}

/// The draft after `action` on a sheet showing `view`, and what accountd is told about it. An
/// action that does not belong to the step on screen changes nothing.
fn act(view: &SheetView, draft: Draft, action: SheetAction) -> (Draft, Option<SheetInput>) {
    match (view, action) {
        (_, SheetAction::Cancel) => (draft, Some(SheetInput::Dismiss)),
        (
            SheetView::SignIn(_) | SheetView::Review(_) | SheetView::Failed { .. },
            SheetAction::Back,
        ) => (draft, Some(SheetInput::Back)),
        (SheetView::Failed { .. }, SheetAction::Retry) => (draft, Some(SheetInput::Retry)),
        (SheetView::BrowserWait { .. }, SheetAction::OpenAgain) => {
            (draft, Some(SheetInput::OpenAgain))
        }
        (SheetView::BrowserWait { .. } | SheetView::ShowCode { .. }, SheetAction::Copied) => (
            Draft {
                copied: CopyMark::Copied,
                ..draft
            },
            None,
        ),
        (SheetView::Providers(rows), action) => providers(rows, draft, action),
        (SheetView::SignIn(form_view), action) => form(&form_view.fields, draft, action),
        (SheetView::Consent(ask), action) => consent(&ask.accounts, draft, action),
        (SheetView::Review(_), SheetAction::Toggle(kind, toggle)) => (
            Draft {
                choices: toggled(&draft.choices, kind, toggle),
                ..draft
            },
            None,
        ),
        (SheetView::Review(_), SheetAction::Confirm) => {
            let choices = draft.choices.clone();
            (draft, Some(SheetInput::Confirm(choices)))
        }
        _ => (draft, None),
    }
}

fn providers(
    rows: &[porter_sheet::ProviderRow],
    draft: Draft,
    action: SheetAction,
) -> (Draft, Option<SheetInput>) {
    match action {
        SheetAction::Query(query) => (Draft { query, ..draft }, None),
        SheetAction::Cursor(key) => (
            Draft {
                cursor: Some(key),
                ..draft
            },
            None,
        ),
        SheetAction::Pick(key) => {
            let picked = match key {
                ListKey::Provider(id) => rows.iter().find(|row| row.id == id).map(|row| &row.id),
                // "Other..." is the mail form, chosen by id: the row order is porter's to change.
                ListKey::Other => rows
                    .iter()
                    .find(|row| row.kind == RowKind::Generic && row.id.as_str() == OTHER_PROVIDER)
                    .map(|row| &row.id),
            }
            .cloned();
            (draft, picked.map(SheetInput::Pick))
        }
        _ => (draft, None),
    }
}

fn form(specs: &[FieldSpec], draft: Draft, action: SheetAction) -> (Draft, Option<SheetInput>) {
    match action {
        SheetAction::Type(kind, value) => {
            let fields = draft
                .fields
                .into_iter()
                .map(|mut field| {
                    // A value of the other sort than the field holds is not a keystroke of it.
                    let same = matches!(
                        (&field.value, &value),
                        (FieldValue::Plain(_), FieldValue::Plain(_))
                            | (FieldValue::Secret(_), FieldValue::Secret(_))
                    );
                    if field.kind == kind && same {
                        field.value = value.clone();
                    }
                    field
                })
                .collect();
            let draft = Draft { fields, ..draft };
            // What decides the shape of the server form reshapes it: the protocol (which fields
            // there are) and the securities (which port is the usual one).
            let reshapes = matches!(
                kind,
                FieldKind::Protocol | FieldKind::Security | FieldKind::OutgoingSecurity
            );
            (
                match reshapes {
                    true => draft.refitted(specs),
                    false => draft,
                },
                None,
            )
        }
        SheetAction::Submit => submit(specs, draft),
        _ => (draft, None),
    }
}

/// Send the form when every required field has text; the secrets leave the draft with it.
fn submit(specs: &[FieldSpec], draft: Draft) -> (Draft, Option<SheetInput>) {
    let ready = draft
        .fields
        .iter()
        .all(|field| field.presence == Presence::Optional || !field.is_blank());
    // Nothing goes while an answer cannot be right either (the same check accountd's machine
    // makes again).
    if !ready || draft.problem(specs).is_some() {
        return (draft, None);
    }
    let answers = draft.answers();
    let fields = draft
        .fields
        .into_iter()
        .map(|mut field| {
            if let FieldValue::Secret(_) = field.value {
                field.value = FieldValue::Secret(porter_core::SecretText::new(""));
            }
            field
        })
        .collect();
    (
        Draft {
            fields,
            submits: SubmitCount(draft.submits.0 + 1),
            ..draft
        },
        Some(SheetInput::Submit(answers)),
    )
}

fn consent(
    accounts: &[porter_core::consent::AccountChoice],
    draft: Draft,
    action: SheetAction,
) -> (Draft, Option<SheetInput>) {
    let ids: Vec<AccountId> = accounts
        .iter()
        .map(|choice| choice.account.clone())
        .collect();
    let answer = |scope: GrantScope| {
        chosen_among(&draft, &ids)
            .map(|account| SheetInput::Answer(ConsentAnswer::Allow { account, scope }))
    };
    match action {
        SheetAction::Choose(account) => (
            Draft {
                chosen: Some(account).filter(|account| ids.contains(account)),
                ..draft
            },
            None,
        ),
        SheetAction::Allow(scope) => {
            let input = answer(scope);
            (draft, input)
        }
        SheetAction::Deny => (draft, Some(SheetInput::Answer(ConsentAnswer::Deny))),
        SheetAction::AddAccount => (draft, Some(SheetInput::Answer(ConsentAnswer::AddAccount))),
        _ => (draft, None),
    }
}
