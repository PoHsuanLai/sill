//! `IdleDim`: the pre-screen-off dim overlay and the driver of its fade.

pub(crate) mod drive;
pub(crate) mod model;
pub(crate) mod view;

pub use model::IdleDimPhase;
pub use view::IdleDim;
