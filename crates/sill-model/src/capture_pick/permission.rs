//! The permission question: an app asks (through the portal backend) whether the person allows
//! something, and the sheet answers with one of two buttons or by being closed. The words are an
//! app's, so each is cleaned and cut here, once, before anything holds or draws them.
//! Pure.

use super::step::AskId;
use crate::untrusted::for_display;

/// The longest app name taken, in characters.
pub const LABEL_MAX_CHARS: usize = 128;
/// The longest headline.
pub const TITLE_MAX_CHARS: usize = 128;
/// The longest line under the headline.
pub const SUBTITLE_MAX_CHARS: usize = 256;
/// The longest explanation.
pub const BODY_MAX_CHARS: usize = 1024;
/// The longest button label: a button is one short word or two.
pub const BUTTON_MAX_CHARS: usize = 32;

/// How many characters of a text are looked at before it is cut: a call can carry megabytes, and
/// only the first few thousand can matter once the leading blanks are gone.
const SCAN_MAX_CHARS: usize = 8 * BODY_MAX_CHARS;

/// The yes button when the app names none.
const GRANT_WORDS: &str = "Allow";
/// The no button when the app names none.
const DENY_WORDS: &str = "Don\u{2019}t Allow";

/// An app's question, in words made safe to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    /// The app's name as the person knows it.
    pub app_label: String,
    /// The headline.
    pub title: String,
    /// The line under the headline.
    pub subtitle: String,
    /// The explanation.
    pub body: String,
    /// The yes button (the default one).
    pub grant_label: String,
    /// The no button.
    pub deny_label: String,
}

impl PermissionAsk {
    /// The question a call carries: every text on one line with control and direction marks
    /// dropped and cut to its ceiling, and the buttons' own words when the call named none.
    pub fn from_wire(
        app_label: &str,
        title: &str,
        subtitle: &str,
        body: &str,
        grant_label: &str,
        deny_label: &str,
    ) -> PermissionAsk {
        let button = |text: &str, fallback: &str| match line(text, BUTTON_MAX_CHARS) {
            words if words.is_empty() => fallback.to_owned(),
            words => words,
        };
        PermissionAsk {
            app_label: line(app_label, LABEL_MAX_CHARS),
            title: line(title, TITLE_MAX_CHARS),
            subtitle: line(subtitle, SUBTITLE_MAX_CHARS),
            body: line(body, BODY_MAX_CHARS),
            grant_label: button(grant_label, GRANT_WORDS),
            deny_label: button(deny_label, DENY_WORDS),
        }
    }
}

/// `text` as one trimmed line: line breaks and tabs become spaces, other control and direction
/// marks go, and it is cut to `max` characters.
fn line(text: &str, max: usize) -> String {
    let spaced: String = text
        .chars()
        .take(SCAN_MAX_CHARS)
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    for_display(spaced.trim(), max)
}

/// The question on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsking {
    pub id: AskId,
    pub ask: PermissionAsk,
}

/// How the person ended the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcome {
    /// Pressed the yes button.
    Grant,
    /// Pressed the no button.
    Deny,
    /// The sheet went away without an answer.
    Closed,
}

impl PermissionOutcome {
    /// The number `AskPermission` returns: 0 granted, 1 denied, 2 closed without answering.
    pub fn wire(self) -> u32 {
        match self {
            PermissionOutcome::Grant => 0,
            PermissionOutcome::Deny => 1,
            PermissionOutcome::Closed => 2,
        }
    }
}
