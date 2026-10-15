//! Which Space a window dragged in Mission Control is over, per output, and how a tile draws it.
//! Pure.

use std::collections::BTreeMap;

use ds_core::vocab::DropState;
use porter_core::SpaceId;

/// The Space under the dragged window on each output that has one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarDrops(BTreeMap<String, SpaceId>);

impl BarDrops {
    /// The same drops with `output`'s changed to `space`, or taken away for `None`.
    pub fn with(mut self, output: &str, space: Option<SpaceId>) -> BarDrops {
        match space {
            Some(space) => self.0.insert(output.to_owned(), space),
            None => self.0.remove(output),
        };
        self
    }

    /// The tile of `space` on `output`: lit while the dragged window is over it.
    pub fn state_of(&self, output: Option<&str>, space: &SpaceId) -> DropState {
        match output.and_then(|name| self.0.get(name)) {
            Some(over) if over == space => DropState::Target,
            _ => DropState::Idle,
        }
    }
}
