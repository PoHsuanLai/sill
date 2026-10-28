//! A request to do an action: which one, and what it is done to.

use serde::{Deserialize, Serialize};
use sill_launcher::actions::ActionId;
use sill_launcher::{AppId, DesktopAction, SettingsTarget, WindowKey};

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

/// How an app is started when that is more than its own `Exec`: one of its desktop actions
/// (New Window), or with addresses to hand it (the Trash's `trash:///`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Start {
    pub app: AppId,
    pub action: DesktopAction,
    /// Dropped URIs (`%u`/`%U`).
    pub uris: Vec<String>,
}

impl Start {
    /// A plain start of `app`: its own `Exec`, nothing handed to it.
    pub fn plain(app: AppId) -> Start {
        Start {
            app,
            action: DesktopAction::Default,
            uris: Vec::new(),
        }
    }
}

/// What an action is done to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Subject {
    None,
    App(AppId),
    /// An app, started some particular way.
    Start(Start),
    Windows(Vec<WindowKey>),
    #[serde(with = "sill_launcher::settings_target::text")]
    Settings(SettingsTarget),
}

impl Subject {
    /// Whether this is what an action that `needs` is given: a place is optional.
    pub fn fits(&self, needs: Needs) -> bool {
        matches!(
            (self, needs),
            (Subject::None, Needs::Nothing | Needs::Place)
                | (Subject::App(_) | Subject::Start(_), Needs::App)
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
        let needs = action.needs;
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
