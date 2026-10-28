//! Automatic for one helper: the person lets the machine choose a model for it, among the models
//! they tick for that helper alone. Nothing is ticked for them and no model is chosen until they
//! confirm. The pool is a setting kept outside this screen; this holds the ticks until they are
//! sent (`HelpersCommand::SetPool`) and says how the sending came out.

use std::collections::BTreeSet;

use porter_infer::ModelRef;

use super::model::{HelpersModel, Open};
use super::models::ModelOffer;
use super::msg::HelpersCommand;
use super::words;

/// Whether the person is choosing the models Automatic may use for the open helper.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Automatic {
    /// Not choosing.
    #[default]
    Off,
    /// Choosing: these are ticked.
    Choosing(BTreeSet<ModelRef>),
}

impl Automatic {
    /// Whether `offer` is ticked.
    pub fn includes(&self, offer: &ModelOffer) -> bool {
        match (self, offer.model()) {
            (Automatic::Choosing(pool), Some(model)) => pool.contains(model),
            (Automatic::Choosing(_), None) | (Automatic::Off, _) => false,
        }
    }
}

/// Start choosing, on the card of a helper that exists (a helper not made yet has no pool).
pub(super) fn start(model: HelpersModel) -> HelpersModel {
    match model.open {
        Open::Card(_) => HelpersModel {
            automatic: Automatic::Choosing(BTreeSet::new()),
            notice: None,
            ..model
        },
        Open::Nothing | Open::Reading(_) | Open::New(_) => model,
    }
}

/// Tick `chosen` if it was not ticked, untick it if it was.
pub(super) fn toggle(model: HelpersModel, chosen: ModelRef) -> HelpersModel {
    match model.automatic {
        Automatic::Choosing(mut pool) => {
            if !pool.remove(&chosen) {
                pool.insert(chosen);
            }
            HelpersModel {
                automatic: Automatic::Choosing(pool),
                ..model
            }
        }
        Automatic::Off => model,
    }
}

/// Give up choosing.
pub(super) fn cancel(model: HelpersModel) -> HelpersModel {
    HelpersModel {
        automatic: Automatic::Off,
        ..model
    }
}

/// Send what is ticked; with nothing ticked, say so and send nothing.
pub(super) fn confirm(model: HelpersModel) -> (HelpersModel, Vec<HelpersCommand>) {
    let (Automatic::Choosing(pool), Open::Card(card)) = (&model.automatic, &model.open) else {
        return (model, Vec::new());
    };
    if pool.is_empty() {
        return (
            HelpersModel {
                notice: Some(words::TICK_A_MODEL.to_owned()),
                ..model
            },
            Vec::new(),
        );
    }
    let command = HelpersCommand::SetPool {
        helper: card.view.line.id.clone(),
        pool: pool.clone(),
    };
    (model, vec![command])
}

/// The pool was sent: Automatic closes with a sentence about how it came out (`saved`: the pool
/// is kept).
pub(super) fn written(model: HelpersModel, saved: bool) -> HelpersModel {
    let sentence = match saved {
        true => words::AUTOMATIC_ON,
        false => words::AUTOMATIC_NOT_SAVED,
    };
    HelpersModel {
        automatic: Automatic::Off,
        notice: Some(sentence.to_owned()),
        ..model
    }
}
