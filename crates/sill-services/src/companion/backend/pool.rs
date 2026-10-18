//! Keeping the models Automatic may use for one helper (`ai.auto.pool.<slot>.helper.<id>`).
//!
//! Nothing the shell may write reaches that key yet: inferd's settings module takes writes only
//! from the Settings app, and neither it nor Settings has a place for a helper's pool. Until one
//! lands, the pool is not kept and the person is told so, rather than seeing Automatic switched
//! on that is not.

use prov::HelperId;
use sill_model::companion::{HelpersMsg, PoolWritten};

/// The pool of `helper`, kept; says that it could not be.
pub(super) fn keep(helper: &HelperId, models: usize) -> HelpersMsg {
    eprintln!(
        "sill: companion: helpers: no way yet to keep the {models} models Automatic may use for {helper}"
    );
    HelpersMsg::PoolWritten(PoolWritten::NotSaved)
}
