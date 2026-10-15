//! Which tile a window dragged in Mission Control is over, per output, and how a tile draws it.
//! Pure.

use std::collections::{BTreeMap, BTreeSet};

use casement_protocols::vocab::Shown;
use ds_core::vocab::DropState;
use porter_core::SpaceId;

/// The tile under the dragged window on each output that has one: a Space's, or the plus that
/// makes a new Space; and the outputs where a window drag is under way, whose every tile accepts
/// the window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarDrops {
    spaces: BTreeMap<String, SpaceId>,
    new_space: BTreeSet<String>,
    dragging: BTreeSet<String>,
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

    /// The same drops with a window drag under way on `output` while `drag` is `Shown`, over
    /// otherwise.
    pub fn with_dragging(mut self, output: &str, drag: Shown) -> BarDrops {
        match drag {
            Shown::Shown => self.dragging.insert(output.to_owned()),
            _ => self.dragging.remove(output),
        };
        self
    }

    /// What a tile shows on `output` when the dragged window is not over it: that it accepts the
    /// window while a drag is under way, nothing otherwise.
    fn resting(&self, output: Option<&str>) -> DropState {
        match output.is_some_and(|name| self.dragging.contains(name)) {
            true => DropState::Accepts,
            false => DropState::Idle,
        }
    }

    /// The tile of `space` on `output`: lit while the dragged window is over it, outlined as
    /// accepting while a drag is under way elsewhere.
    pub fn state_of(&self, output: Option<&str>, space: &SpaceId) -> DropState {
        match output.and_then(|name| self.spaces.get(name)) {
            Some(over) if over == space => DropState::Target,
            _ => self.resting(output),
        }
    }

    /// The plus on `output`: lit while the dragged window is over it, outlined as accepting while
    /// a drag is under way elsewhere.
    pub fn new_state_of(&self, output: Option<&str>) -> DropState {
        match output.is_some_and(|name| self.new_space.contains(name)) {
            true => DropState::Target,
            false => self.resting(output),
        }
    }
}
