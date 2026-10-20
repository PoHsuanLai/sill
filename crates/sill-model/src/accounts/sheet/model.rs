//! The state types (data only).

use porter_core::AccountId;
use porter_core::Toggle;
use porter_core::sheet::{ServiceChoice, SheetView};

use super::draft::{CopyMark, FieldDraft, ListKey, SubmitCount};
use crate::serve::ServeLink;

/// The most sheets held at once: accountd opens one per request and waits on it, so a queue
/// longer than this is something other than accountd.
pub const MAX_OPEN: usize = 8;

/// The longest handle taken, in bytes (accountd's are `accountd-<n>`).
const MAX_HANDLE: usize = 64;

/// What accountd calls one sheet: the first argument of `Open`, `Update`, `Close` and the `Input`
/// signal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SheetHandle(String);

impl SheetHandle {
    /// The handle written as `text`: one to 64 characters of `[A-Za-z0-9_.-]`, so it is safe in a
    /// log line and a test's name for it. `None` for anything else.
    pub fn parse(text: &str) -> Option<SheetHandle> {
        let fits = (1..=MAX_HANDLE).contains(&text.len())
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
        fits.then(|| SheetHandle(text.to_owned()))
    }

    /// The handle's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether the shell serves `org.quire.AccountsSheet1`.
pub type SheetLink = ServeLink;

/// What the person has done on a sheet that is not yet sent: every value the sheet shows that the
/// shell owns. Rebuilt from the view each time the view changes ([`super::draft`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Draft {
    /// What the provider search holds.
    pub query: String,
    /// The provider row under the arrow keys.
    pub cursor: Option<ListKey>,
    /// The sign-in form's fields with what they hold. A secret is held here and nowhere else, and
    /// emptied when the form is sent.
    pub fields: Vec<FieldDraft>,
    /// How many times the form has been sent.
    pub submits: SubmitCount,
    /// The account the consent alert would use; `None` until one is picked (the first then).
    pub chosen: Option<AccountId>,
    /// The review's switches.
    pub choices: Vec<ServiceChoice>,
    /// Whether the link or code on screen has been copied.
    pub copied: CopyMark,
}

impl Draft {
    /// The review's switch for `kind`, when the provider offers it.
    pub fn toggle_of(&self, kind: porter_core::CapabilityKind) -> Option<Toggle> {
        self.choices
            .iter()
            .find(|choice| choice.kind == kind)
            .map(|choice| choice.toggle)
    }
}

/// One sheet accountd opened: what it shows now, and what is typed on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSheet {
    /// Its handle.
    pub handle: SheetHandle,
    /// What accountd last said to draw.
    pub view: SheetView,
    /// What the person has done on it.
    pub draft: Draft,
    /// Whether an answer has gone to accountd that it has not yet followed with a new view.
    pub awaiting: Awaiting,
}

/// Whether accountd owes the sheet a new view. A press (or Return, or a double-reported click)
/// that has been answered must not be answered twice: accountd's machine moves on one input, and
/// a second one would be taken for the next step's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Awaiting {
    /// Nothing is owed: the next answer goes.
    #[default]
    Nothing,
    /// An answer went; only a dismissal may follow until accountd updates the sheet.
    View,
}

impl OpenSheet {
    /// Whether there is anything to draw: a finished add shows nothing while accountd closes it.
    pub fn visible(&self) -> bool {
        !matches!(self.view, SheetView::Done)
    }
}

/// The sheets, published by the service. The first is the one on screen; the rest wait their turn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountsSheets {
    /// Whether the shell serves the sheet.
    pub link: SheetLink,
    /// The open sheets, in the order accountd opened them.
    pub open: Vec<OpenSheet>,
}

impl AccountsSheets {
    /// The sheet on screen.
    pub fn current(&self) -> Option<&OpenSheet> {
        self.open.first().filter(|sheet| sheet.visible())
    }
}
