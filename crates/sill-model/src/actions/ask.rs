//! A request to do an action: which one, and what it is done to.

use serde::{Deserialize, Serialize};
use sill_launcher::actions::ActionId;
use sill_launcher::{AppId, SettingsLink, WindowKey};

use super::def;

/// What an action must be told to do it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    /// Nothing: it acts on the desktop.
    Nothing,
    /// An app.
    App,
    /// An app's windows, by the key each has.
    Windows,
    /// A place in Settings, or none (Settings opens where it was).
    Place,
}

/// What an action is done to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Subject {
    None,
    App(AppId),
    Windows(Vec<WindowKey>),
    Settings(SettingsLink),
}

impl Subject {
    /// Whether this is what an action that `needs` is given: a place is optional.
    pub fn fits(&self, needs: Needs) -> bool {
        matches!(
            (self, needs),
            (Subject::None, Needs::Nothing | Needs::Place)
                | (Subject::App(_), Needs::App)
                | (Subject::Windows(_), Needs::Windows)
                | (Subject::Settings(_), Needs::Place)
        )
    }
}

/// One request to do an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    pub action: ActionId,
    pub subject: Subject,
}

impl Ask {
    /// `action` on the desktop itself.
    pub fn of(action: ActionId) -> Ask {
        Ask {
            action,
            subject: Subject::None,
        }
    }

    /// `action` done to `subject`.
    pub fn on(action: ActionId, subject: Subject) -> Ask {
        Ask { action, subject }
    }

    /// Why the request cannot be done as asked, in words for a person: the action was given the
    /// wrong thing to act on.
    pub fn refusal(&self) -> Option<String> {
        let action = def(self.action);
        let needs = action.effect.needs();
        if self.subject.fits(needs) {
            return None;
        }
        let what = match needs {
            Needs::Nothing => "takes nothing to act on",
            Needs::App => "needs an app",
            Needs::Windows => "needs an app's windows",
            Needs::Place => "takes a place in Settings, or none",
        };
        Some(format!("{} {what}.", action.label))
    }
}
