//! The app switcher (design/13 §13.3.5): not a milestone M1-M4 surface; its pure machine is
//! frozen now (design/13 §13.4 places it at `surfaces/switcher/machine.rs`).

pub mod machine;

pub use machine::{Dir, SwIn, SwKey, SwOut, Switcher, SwitcherParams, step};
