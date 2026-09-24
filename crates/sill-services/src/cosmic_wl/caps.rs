//! What the compositor says a workspace accepts: the ext-workspace capabilities and COSMIC's
//! own, merged into one typed set so a surface can grey out what it cannot ask for.

use serde::{Deserialize, Serialize};

/// One request a workspace accepts.
///
/// The derived order is only the set's canonical form (declaration order); it means nothing
/// else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCap {
    /// ext-workspace `activate`.
    Activate,
    /// ext-workspace `deactivate`.
    Deactivate,
    /// ext-workspace `remove`.
    Remove,
    /// ext-workspace `assign` (to another group).
    Assign,
    /// COSMIC `rename`.
    Rename,
    /// COSMIC `set_tiling_state`.
    SetTiling,
    /// COSMIC `pin`/`unpin` (`WlCommand::PinWorkspace`, `UnpinWorkspace`).
    Pin,
    /// COSMIC `move_before`/`move_after` (`WlCommand::MoveWorkspace`).
    Move,
}

/// A set of [`WorkspaceCap`]s: sorted and without repeats, however it was built, so two sets
/// with the same members are equal. Empty when the compositor said nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(from = "Vec<WorkspaceCap>", into = "Vec<WorkspaceCap>")]
pub struct WorkspaceCaps(Vec<WorkspaceCap>);

impl WorkspaceCaps {
    /// Whether the workspace accepts `cap`.
    pub fn contains(&self, cap: WorkspaceCap) -> bool {
        self.0.binary_search(&cap).is_ok()
    }

    /// The members, in canonical order.
    pub fn iter(&self) -> impl Iterator<Item = WorkspaceCap> + '_ {
        self.0.iter().copied()
    }
}

impl FromIterator<WorkspaceCap> for WorkspaceCaps {
    fn from_iter<I: IntoIterator<Item = WorkspaceCap>>(caps: I) -> Self {
        let mut all: Vec<WorkspaceCap> = caps.into_iter().collect();
        all.sort();
        all.dedup();
        WorkspaceCaps(all)
    }
}

impl From<Vec<WorkspaceCap>> for WorkspaceCaps {
    fn from(caps: Vec<WorkspaceCap>) -> Self {
        caps.into_iter().collect()
    }
}

impl From<WorkspaceCaps> for Vec<WorkspaceCap> {
    fn from(caps: WorkspaceCaps) -> Self {
        caps.0
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceCap, WorkspaceCaps};

    #[test]
    fn a_set_is_canonical_however_it_was_built() {
        use WorkspaceCap::{Activate, Move, Pin};
        let cases: &[(&str, &[WorkspaceCap], &[WorkspaceCap])] = &[
            ("empty", &[], &[]),
            ("sorted", &[Move, Activate], &[Activate, Move]),
            ("deduplicated", &[Pin, Pin, Activate], &[Activate, Pin]),
        ];
        for (name, given, want) in cases {
            let set: WorkspaceCaps = given.iter().copied().collect();
            assert_eq!(set.iter().collect::<Vec<_>>(), *want, "{name}");
            for cap in *want {
                assert!(set.contains(*cap), "{name}: {cap:?}");
            }
        }
        let a: WorkspaceCaps = [Move, Activate].into_iter().collect();
        let b: WorkspaceCaps = [Activate, Move, Move].into_iter().collect();
        assert_eq!(a, b);
        assert!(!a.contains(Pin));
    }

    #[test]
    fn the_wire_form_is_a_list_and_decoding_normalises_it() {
        let set: WorkspaceCaps = [WorkspaceCap::Pin, WorkspaceCap::Activate]
            .into_iter()
            .collect();
        let line = serde_json::to_string(&set).unwrap_or_default();
        assert_eq!(line, r#"["activate","pin"]"#);
        let back: WorkspaceCaps =
            serde_json::from_str(r#"["pin","activate","pin"]"#).unwrap_or_default();
        assert_eq!(back, set);
    }
}
