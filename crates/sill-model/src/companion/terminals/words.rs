//! The words a session in a terminal wears in the panel.

use companion_wire::TerminalSession;

use crate::companion::helpers::program_name;

/// The button that brings a session back from its terminal.
pub const BRING_BACK_LABEL: &str = "Bring Back";

/// Said when the companion could not be asked to bring a session back.
pub const NO_BRING_BACK: &str = "That session could not be brought back just now.";

/// "In Terminal \u{2014} Claude Code": where the session is, and the app that runs it.
pub fn session_title(session: &TerminalSession) -> String {
    let app = session
        .app_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map_or_else(|| program_name(session.program.as_str()), str::to_owned);
    format!("In Terminal \u{2014} {app}")
}

/// The folder the session works in, or nothing.
pub fn session_subtitle(session: &TerminalSession) -> String {
    session.folder.clone().unwrap_or_default()
}
