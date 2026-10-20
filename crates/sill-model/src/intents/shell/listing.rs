//! The text of `shell.window.list` and `shell.workspace.list`: one line each, `name=value`
//! fields. Window titles and app ids are the apps' words, so they are squeezed onto one line
//! with no quote in them and cut short; the listing is labelled untrusted by the service.

use crate::units::Urgency;
use crate::windows::{
    Activity, Sizing, Toplevel, WindowModel, WindowState, Workspace, WorkspaceGroup, Workspaces,
};

/// The most characters the answer carries (the manifest's `max`).
pub const LISTING_MAX: usize = 12_000;

/// The longest title or app id kept in a line.
const FIELD_MAX: usize = 120;

/// One line per window, most recently active first.
pub fn windows_text(wl: &WindowModel) -> String {
    let lines = wl
        .toplevels
        .windows
        .iter()
        .map(|w| window_line(w, &wl.workspaces));
    capped(&lines.collect::<Vec<_>>(), "windows")
}

/// One line per workspace of the main group, in order.
pub fn workspaces_text(workspaces: &Workspaces) -> String {
    let lines = workspaces
        .main_group()
        .map(|g| {
            g.workspaces
                .iter()
                .zip(1..)
                .map(|(w, at)| workspace_line(w, at))
        })
        .into_iter()
        .flatten();
    capped(&lines.collect::<Vec<_>>(), "workspaces")
}

fn window_line(window: &Toplevel, workspaces: &Workspaces) -> String {
    let state = match (window.window, window.activity) {
        (WindowState::Minimized, _) => "minimised",
        (WindowState::Normal, Activity::Active) => "active",
        (WindowState::Normal, Activity::Inactive) => "normal",
    };
    let size = match window.sizing {
        Sizing::Floating => "floating",
        Sizing::Maximized => "maximised",
        Sizing::Fullscreen => "fullscreen",
    };
    let at = window
        .workspaces
        .first()
        .and_then(|key| position_of(workspaces, key.0))
        .map_or_else(|| "-".to_owned(), |at| at.to_string());
    format!(
        "id={} app={} workspace={at} state={state} size={size} title=\"{}\"",
        window.key.0,
        one_word(&window.app_id),
        one_line(&window.title),
    )
}

/// The position (from 1) of workspace `key` in its group.
fn position_of(workspaces: &Workspaces, key: u64) -> Option<usize> {
    let group: &WorkspaceGroup = workspaces
        .groups
        .iter()
        .find(|g| g.workspaces.iter().any(|w| w.key.0 == key))?;
    group
        .workspaces
        .iter()
        .position(|w| w.key.0 == key)
        .map(|at| at + 1)
}

fn workspace_line(workspace: &Workspace, index: usize) -> String {
    let yes_no = |on: bool| if on { "yes" } else { "no" };
    format!(
        "index={index} id={} name=\"{}\" active={} urgent={}",
        workspace.key.0,
        one_line(&workspace.name),
        yes_no(workspace.activity == Activity::Active),
        yes_no(workspace.urgency == Urgency::Urgent),
    )
}

/// `text` on one line: control characters become spaces and a double quote a single one, cut at
/// [`FIELD_MAX`] characters.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' => '\'',
            c if c.is_control() => ' ',
            c => c,
        })
        .take(FIELD_MAX)
        .collect()
}

/// `text` with no whitespace at all, for a field that is not quoted.
fn one_word(text: &str) -> String {
    one_line(text)
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// `lines` joined, stopping before [`LISTING_MAX`] and saying how many were left out.
fn capped(lines: &[String], noun: &str) -> String {
    if lines.is_empty() {
        return format!("(no {noun})\n");
    }
    let mut text = String::new();
    for (shown, line) in lines.iter().enumerate() {
        let left = lines.len() - shown;
        let note = format!("... and {left} more {noun}\n");
        if text.chars().count() + line.chars().count() + 1 + note.chars().count() > LISTING_MAX {
            text.push_str(&note);
            return text;
        }
        text.push_str(line);
        text.push('\n');
    }
    text
}
