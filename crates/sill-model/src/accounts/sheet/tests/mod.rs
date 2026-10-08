mod step;

use porter_core::ProviderId;
use porter_core::sheet::{
    Entry, FieldKind, FieldSpec, Presence, ProviderRow, RowKind, SheetView, SignInView,
};

use super::SheetHandle;

pub(super) fn handle(text: &str) -> SheetHandle {
    SheetHandle::parse(text).unwrap_or_else(|| panic!("{text}"))
}

pub(super) fn provider(id: &str) -> ProviderId {
    ProviderId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

pub(super) fn rows(ids: &[&str]) -> SheetView {
    SheetView::Providers(
        ids.iter()
            .map(|id| ProviderRow {
                id: provider(id),
                label: (*id).to_owned(),
                mark: (*id).to_owned(),
                kind: if id.starts_with("generic-") {
                    RowKind::Generic
                } else {
                    RowKind::Provider
                },
                auth: porter_core::sheet::ProviderKind::Service,
                mark_face: None,
                group: None,
            })
            .collect(),
    )
}

pub(super) fn spec(kind: FieldKind, entry: Entry, presence: Presence) -> FieldSpec {
    FieldSpec {
        kind,
        entry,
        presence,
        prefill: None,
    }
}

/// An address and a password, both required.
pub(super) fn form(prefill: Option<&str>) -> SheetView {
    let mut address = spec(FieldKind::Address, Entry::Plain, Presence::Required);
    address.prefill = prefill.map(str::to_owned);
    SheetView::SignIn(SignInView {
        provider: provider("nextcloud"),
        row: None,
        fields: vec![
            address,
            spec(FieldKind::Password, Entry::Secret, Presence::Required),
        ],
        problem: None,
    })
}
