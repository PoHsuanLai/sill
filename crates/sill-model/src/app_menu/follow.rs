//! Which apps' menus are followed: the union of what each bar asks for.

use std::collections::{BTreeMap, BTreeSet};

use super::{Follower, MenuApp};

/// What each follower wants followed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Followers {
    wants: BTreeMap<Follower, MenuApp>,
}

/// What changed in the set of followed apps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Following {
    /// Apps no follower named before and one names now.
    pub start: Vec<MenuApp>,
    /// Apps a follower named before and none names now.
    pub stop: Vec<MenuApp>,
}

impl Followers {
    /// The apps some follower names.
    pub fn apps(&self) -> BTreeSet<&MenuApp> {
        self.wants.values().collect()
    }

    /// `by` now wants `app` (nothing, for `None`): the new wants, and which apps start and stop
    /// being followed.
    pub fn set(self, by: Follower, app: Option<MenuApp>) -> (Followers, Following) {
        let before: BTreeSet<MenuApp> = self.wants.values().cloned().collect();
        let mut wants = self.wants;
        match app {
            Some(app) => wants.insert(by, app),
            None => wants.remove(&by),
        };
        let next = Followers { wants };
        let after: BTreeSet<MenuApp> = next.wants.values().cloned().collect();
        let following = Following {
            start: after.difference(&before).cloned().collect(),
            stop: before.difference(&after).cloned().collect(),
        };
        (next, following)
    }
}
