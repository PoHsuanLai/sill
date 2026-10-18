//! Every card's footer: which model served it, what it drew on, what the prompt carried.

use companion_wire::FooterWire;
use docket_core::{ContextKeep, EntityRef, Keep};
use ds::components::companion::answer::footer::{
    CardFooter, ServedByView, ServedPlace, ServedPlaceKind, SourceChip, SourceKey,
};
use ds::components::companion::{ChipKind, ContextChip, Removal};
use ds::components::content::icon_source::IconSource;
use ds::prelude::Icon;
use porter_core::Locality;

/// The footer of a card.
pub fn footer_view(footer: &FooterWire) -> CardFooter {
    CardFooter {
        served_by: footer
            .served
            .first()
            .map_or_else(unknown_model, |served| ServedByView {
                model: served.model.as_str().to_owned(),
                place: place_of(&served.locality, served.account.as_str()),
            }),
        sources: footer.sources.iter().map(source_chip).collect(),
        scope: scope_chips(footer.keep),
    }
}

/// The model that answered is not known yet (an answer still being thought of).
fn unknown_model() -> ServedByView {
    ServedByView {
        model: String::new(),
        place: ServedPlace {
            kind: ServedPlaceKind::ThisComputer,
            provider: None,
        },
    }
}

/// A thing the answer drew on, as a chip that opens it: the key the shell opens is the thing's
/// app, kind and key.
fn source_chip(source: &EntityRef) -> SourceChip {
    SourceChip {
        label: source.title.value.clone(),
        icon: IconSource::Glyph(Icon::Inbox),
        key: SourceKey(format!(
            "{}:{}:{}",
            source.id.app.as_str(),
            source.id.kind.as_str(),
            source.id.key.as_str()
        )),
    }
}

/// The chips of context the person kept, in the order they are offered.
fn scope_chips(keep: ContextKeep) -> Vec<ContextChip> {
    [
        (keep.query, ChipKind::Query, "Search"),
        (keep.results, ChipKind::Results, "Results"),
        (keep.selection, ChipKind::Selection, "Selection"),
        (keep.window, ChipKind::Window, "Window"),
    ]
    .into_iter()
    .filter(|(kept, _, _)| *kept == Keep::Kept)
    .map(|(_, kind, label)| ContextChip {
        kind,
        label: label.to_owned(),
        count: None,
        removal: Removal::Fixed,
    })
    .collect()
}

/// Where a model ran, as the footer says it.
fn place_of(locality: &Locality, account: &str) -> ServedPlace {
    match locality {
        Locality::OnDevice => ServedPlace {
            kind: ServedPlaceKind::ThisComputer,
            provider: None,
        },
        Locality::LocalNetwork => ServedPlace {
            kind: ServedPlaceKind::LocalNetwork,
            provider: None,
        },
        Locality::Cloud { .. } => ServedPlace {
            kind: ServedPlaceKind::Cloud,
            provider: Some(account.to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::place_of;
    use ds::components::companion::answer::footer::ServedPlaceKind;
    use porter_core::{Locality, Region};

    #[test]
    fn a_model_ran_on_this_computer_on_the_network_or_at_a_provider() {
        let cases = [
            (Locality::OnDevice, ServedPlaceKind::ThisComputer, None),
            (Locality::LocalNetwork, ServedPlaceKind::LocalNetwork, None),
            (
                Locality::Cloud { region: None },
                ServedPlaceKind::Cloud,
                Some("acme"),
            ),
            (
                Locality::Cloud {
                    region: Some(Region("eu-west-1".to_owned())),
                },
                ServedPlaceKind::Cloud,
                Some("acme"),
            ),
        ];
        for (locality, kind, provider) in cases {
            let place = place_of(&locality, "acme");
            assert_eq!(place.kind, kind, "{locality:?}");
            assert_eq!(place.provider.as_deref(), provider, "{locality:?}");
        }
    }
}
