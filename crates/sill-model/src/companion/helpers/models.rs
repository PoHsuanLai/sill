//! The models a helper may be given: the ones the places the assistant could run serve, grouped
//! the way the assistant's own model list groups them (this computer, the person's other
//! computers, each cloud company; this computer's models under the engine that runs them).
//! Nothing is marked or preselected: a helper stays without a model until the person picks one.

use std::collections::HashSet;

use docket_core::helper::HelperModel;
use porter_core::AccountId;
use porter_infer::{ModelRef, PlaceEngine, PlaceKind, PlaceRow};

/// The account the models of this computer and of the person's other computers are named under.
const LOCAL_ACCOUNT: &str = "local";

/// The account the hosted models are named under.
const CLOUD_ACCOUNT: &str = "cloud";

/// One model the person may pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOffer {
    /// What choosing it sets.
    pub choice: HelperModel,
    /// Its name, as the list shows it.
    pub label: String,
}

impl ModelOffer {
    /// The model choosing it names.
    pub fn model(&self) -> Option<&ModelRef> {
        match &self.choice {
            HelperModel::Named(model) => Some(model),
            HelperModel::Unset => None,
        }
    }
}

/// Models that run in one place, under one heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelGroup {
    /// Where they run: "On this computer, with llama.cpp", a computer's name, a company.
    pub heading: String,
    /// The models, in the order the places list them.
    pub offers: Vec<ModelOffer>,
}

/// The list of models on a helper's card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ModelList {
    /// Not shown.
    #[default]
    Closed,
    /// Asked for and not yet here.
    Reading,
    /// Shown; no group at all means no model is available.
    Offered(Vec<ModelGroup>),
}

/// The groups of `places`, a place with no model left out and a model listed once. The models of
/// this computer are grouped under the engine that runs each; a model whose engine is not known
/// goes under a heading without one.
pub fn model_groups(places: &[PlaceRow]) -> Vec<ModelGroup> {
    let mut seen = HashSet::new();
    places
        .iter()
        .flat_map(groups_of)
        .map(|group| ModelGroup {
            offers: group
                .offers
                .into_iter()
                .filter(|offer| seen.insert(offer.choice.clone()))
                .collect(),
            ..group
        })
        .filter(|group| !group.offers.is_empty())
        .collect()
}

/// The groups of one place: one for a cloud account or another computer, one per engine for this
/// computer, in the order the engines first appear.
fn groups_of(place: &PlaceRow) -> Vec<ModelGroup> {
    let account = match place.kind {
        PlaceKind::CloudAccount => CLOUD_ACCOUNT,
        PlaceKind::ThisComputer | PlaceKind::OwnComputer => LOCAL_ACCOUNT,
    };
    let Ok(account) = AccountId::parse(account) else {
        return Vec::new();
    };
    let mut groups: Vec<(Option<&PlaceEngine>, Vec<ModelOffer>)> = Vec::new();
    for model in &place.models {
        let engine = engine_of(place.kind, model.engine.as_ref());
        let offer = ModelOffer {
            choice: HelperModel::Named(ModelRef {
                account: account.clone(),
                model: model.id.clone(),
            }),
            label: model.name.clone(),
        };
        match groups.iter_mut().find(|(known, _)| *known == engine) {
            Some((_, offers)) => offers.push(offer),
            None => groups.push((engine, vec![offer])),
        }
    }
    groups
        .into_iter()
        .map(|(engine, offers)| ModelGroup {
            heading: heading_of(place, engine),
            offers,
        })
        .collect()
}

/// The engine that runs a model, for the places that name one: only this computer's.
fn engine_of(kind: PlaceKind, engine: Option<&PlaceEngine>) -> Option<&PlaceEngine> {
    match kind {
        PlaceKind::ThisComputer => engine,
        PlaceKind::OwnComputer | PlaceKind::CloudAccount => None,
    }
}

fn heading_of(place: &PlaceRow, engine: Option<&PlaceEngine>) -> String {
    match (place.kind, engine) {
        (PlaceKind::ThisComputer, Some(engine)) => {
            format!("On this computer, with {}", engine.name)
        }
        (PlaceKind::ThisComputer, None) => "On this computer".to_owned(),
        (PlaceKind::OwnComputer, _) => format!("On {}", place.name),
        (PlaceKind::CloudAccount, _) => {
            place.provider.clone().unwrap_or_else(|| place.name.clone())
        }
    }
}
