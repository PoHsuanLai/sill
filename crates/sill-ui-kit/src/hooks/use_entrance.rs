//! `use_entrance`: where a component asks the daemon for an action of the shell, and where it
//! says the ask came from. A surface root provides the `ActionLink` and, for a popup, the
//! `Origin` of the surface it hangs from (the popup closes as the person picks, so the daemon
//! asks the parent for the activation token); a headless render has neither and the ask goes
//! nowhere.

use dioxus::prelude::try_use_context;
use sill_launcher::actions::ActionId;
use sill_model::actions::Subject;

use super::use_link::use_link;
use crate::links::{ActionLink, Origin};

/// A component's way to ask for actions. Clone is cheap.
#[derive(Debug, Clone)]
pub struct Entrance {
    link: Option<ActionLink>,
    origin: Origin,
}

impl Entrance {
    /// Ask for `action` on the desktop itself.
    pub fn do_it(&self, action: ActionId) {
        if let Some(link) = &self.link {
            link.do_it(action, self.origin);
        }
    }

    /// Ask for `action` done to `subject`.
    pub fn do_to(&self, action: ActionId, subject: Subject) {
        if let Some(link) = &self.link {
            link.do_to(action, subject, self.origin);
        }
    }
}

/// The entrance of the surface being built.
pub fn use_entrance() -> Entrance {
    Entrance {
        link: use_link::<ActionLink>(),
        origin: try_use_context::<Origin>().unwrap_or_default(),
    }
}
