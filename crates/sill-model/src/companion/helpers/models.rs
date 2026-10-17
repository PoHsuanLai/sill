//! The models a helper may be given: the ones the places the assistant could run serve, grouped
//! the way the assistant's own model list groups them (this computer, the person's other
//! computers, each cloud company). Nothing is marked or preselected: a helper stays without a
//! model until the person picks one.

use std::collections::HashSet;

use docket_core::helper::HelperModel;
use porter_core::AccountId;
use porter_infer::{ModelRef, PlaceKind, PlaceRow};

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

/// The groups of `places`, a place with no model left out and a model listed once. `engines` are
/// the names of the engines on this computer that run text models: with exactly one, it is named
/// in the heading of this computer's group.
pub fn model_groups(places: &[PlaceRow], engines: &[String]) -> Vec<ModelGroup> {
    let mut seen = HashSet::new();
    places
        .iter()
        .filter_map(|place| group_of(place, engines))
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

fn group_of(place: &PlaceRow, engines: &[String]) -> Option<ModelGroup> {
    let account = match place.kind {
        PlaceKind::CloudAccount => CLOUD_ACCOUNT,
        PlaceKind::ThisComputer | PlaceKind::OwnComputer => LOCAL_ACCOUNT,
    };
    let account = AccountId::parse(account).ok()?;
    let offers = place
        .models
        .iter()
        .map(|model| ModelOffer {
            choice: HelperModel::Named(ModelRef {
                account: account.clone(),
                model: model.id.clone(),
            }),
            label: model.name.clone(),
        })
        .collect();
    Some(ModelGroup {
        heading: heading_of(place, engines),
        offers,
    })
}

fn heading_of(place: &PlaceRow, engines: &[String]) -> String {
    match (place.kind, engines) {
        (PlaceKind::ThisComputer, [engine]) => format!("On this computer, with {engine}"),
        (PlaceKind::ThisComputer, _) => "On this computer".to_owned(),
        (PlaceKind::OwnComputer, _) => format!("On {}", place.name),
        (PlaceKind::CloudAccount, _) => {
            place.provider.clone().unwrap_or_else(|| place.name.clone())
        }
    }
}
