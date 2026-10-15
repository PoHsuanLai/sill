//! The stage's pure transition.

use super::model::{SheetStage, StageIn};

/// `stage` after `input`.
pub(super) fn advance<T>(stage: SheetStage<T>, input: StageIn<T>) -> SheetStage<T> {
    match (stage, input) {
        (_, StageIn::Want(Some(value))) => SheetStage::Up(value),
        (SheetStage::Up(value), StageIn::Want(None)) => SheetStage::Leaving(value),
        (SheetStage::Leaving(_), StageIn::Settled) => SheetStage::Empty,
        (stage, StageIn::Want(None) | StageIn::Settled) => stage,
    }
}
