//! Which tile a window dragged in Mission Control is over, per output, and how a tile draws it.
//! Pure.

use std::collections::{BTreeMap, BTreeSet};

use casement_protocols::vocab::Shown;
use ds_core::vocab::DropState;
use porter_core::SpaceId;

/// The tile under the dragged window on each output that has one: a Space's, or the plus that
/// makes a new Space.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarDrops {
    spaces: BTreeMap<String, SpaceId>,
    new_space: BTreeSet<String>,
}

impl BarDrops {
    /// The same drops with `output`'s Space tile changed to `space`, or taken away for `None`. A
    /// Space under the window is not the plus, so the plus goes out.
    pub fn with(mut self, output: &str, space: Option<SpaceId>) -> BarDrops {
        match space {
            Some(space) => {
                self.new_space.remove(output);
                self.spaces.insert(output.to_owned(), space)
            }
            None => self.spaces.remove(output),
        };
        self
    }

    /// The same drops with the plus of `output` lit while `over` is `Shown`, unlit otherwise.
    pub fn with_new(mut self, output: &str, over: Shown) -> BarDrops {
        match over {
            Shown::Shown => self.new_space.insert(output.to_owned()),
            _ => self.new_space.remove(output),
        };
        self
    }

    /// The tile of `space` on `output`: lit while the dragged window is over it.
    pub fn state_of(&self, output: Option<&str>, space: &SpaceId) -> DropState {
        match output.and_then(|name| self.spaces.get(name)) {
            Some(over) if over == space => DropState::Target,
            _ => DropState::Idle,
        }
    }

    /// The plus on `output`: lit while the dragged window is over it.
    pub fn new_state_of(&self, output: Option<&str>) -> DropState {
        match output.is_some_and(|name| self.new_space.contains(name)) {
            true => DropState::Target,
            false => DropState::Idle,
        }
    }
}
