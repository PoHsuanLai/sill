//! What the control center shows of the terminal.

use docket_core::ActionRef;
use serde::{Deserialize, Serialize};

/// Whether intentd's breaker has paused the terminal. It pauses a terminal session after too many
/// refusals in a row and only the person lifts it, so the control center says so and offers the
/// way back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Breaker {
    #[default]
    Running,
    Tripped,
}

/// Whether intentd's `Control` was reachable at the last read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlReach {
    /// Not read yet.
    #[default]
    Unread,
    /// Read.
    Reached,
    /// intentd is not on the bus, or refused the read.
    Unreachable,
}

/// The terminal as the person can see and change it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Terminal {
    /// The actions the terminal may run without asking until logout, in intentd's order.
    pub grants: Vec<ActionRef>,
    pub breaker: Breaker,
    pub reach: ControlReach,
}

/// A grant in words: the action's name, then the app it is of (`thread.archive`, `org.quire.Mail`
/// is "Thread archive" in "Mail"). The name is the manifest's, never the terminal's text.
pub fn grant_words(grant: &ActionRef) -> (String, String) {
    let app = grant.app.as_str();
    let slug = app.rsplit('.').next().unwrap_or(app).to_lowercase();
    let name = grant.name.as_str();
    let rest = name
        .strip_prefix(slug.as_str())
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(name);
    let words = rest.replace(['.', '_', '-'], " ");
    (capitalised(&words), capitalised(&slug))
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// `app:action` (`org.quire.Shell:shell.dock.pin`), the way a person names a grant on the command
/// line; the other way round is [`grant_name`].
pub fn parse_grant(text: &str) -> Result<ActionRef, String> {
    let (app, action) = text
        .split_once(':')
        .ok_or_else(|| format!("{text:?} is not APP:ACTION (org.quire.Shell:shell.lock)"))?;
    let app = prov::AppName::parse(app).map_err(|e| e.to_string())?;
    let name = prov::ActionName::parse(action).map_err(|e| e.to_string())?;
    Ok(ActionRef { app, name })
}

/// A grant as [`parse_grant`] reads it.
pub fn grant_name(grant: &ActionRef) -> String {
    format!("{}:{}", grant.app, grant.name)
}
