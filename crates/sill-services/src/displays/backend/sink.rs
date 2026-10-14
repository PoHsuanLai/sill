//! Where an output's scale is read and set: the [`ScaleSink`] seam. Its real side is
//! [`super::sink_wlr::WlrOutputs`], the compositor's own output management.

use sill_model::displays::randr::Head;
use sill_model::displays::scale::DisplayScale;

use crate::error::ServiceError;

/// The compositor's outputs, and the one change sill makes to them.
pub trait ScaleSink {
    /// Every output the compositor reports now.
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError>;
    /// Run `head` at `scale`, keeping its current mode.
    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError>;
}
