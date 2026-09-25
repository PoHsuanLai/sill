//! Stand-ins for the quire components the control center needs and quire does not have yet
//! (FINDINGS "M5 control center", gaps Q78-Q81). Each is built only from quire components that
//! exist, and named and shaped like the one quire is building (its `cc-parts` branch):
//! [`ModuleTile`] for Q78, [`SettingsRows`] for Q79, [`PaneSwitcher`] for Q80 and [`glyphs`]
//! for Q81. When they land, each switches here, in one place, and the modules and pages above
//! them do not change.

pub(crate) mod glyphs;
mod pane;
mod rows;
mod tile;

pub(crate) use pane::PaneSwitcher;
pub(crate) use rows::{RowMark, SettingsRow, SettingsRows};
pub(crate) use tile::{ModuleState, ModuleTile, TileFrame, TileSpan};
