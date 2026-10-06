//! What the sheet can be told, what can happen to it, and what the shell must then do.

use porter_core::Toggle;
use porter_core::capability::CapabilityKind;
use porter_core::consent::GrantScope;
use porter_core::sheet::{FieldKind, FieldValue, SheetInput, SheetView};
use porter_core::{AccountId, WebUrl};

use super::draft::ListKey;
use super::model::{AccountsSheets, MAX_OPEN, SheetHandle, SheetLink};

/// What the person does on a sheet, as the drawn parts report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetAction {
    /// The provider search changed.
    Query(String),
    /// The arrow keys moved to a provider row.
    Cursor(ListKey),
    /// A provider row was pressed, or Return was pressed on it.
    Pick(ListKey),
    /// A form field changed. Secret text arrives as [`FieldValue::Secret`], whose `Debug` hides it.
    Type(FieldKind, FieldValue),
    /// Continue on the form.
    Submit,
    /// The consent alert's pop-up chose an account.
    Choose(AccountId),
    /// "Allow Once" or "Always Allow" on the consent alert.
    Allow(GrantScope),
    /// "Don't Allow".
    Deny,
    /// "Add Account…" on the consent alert: accountd runs the add sheet and allows the app in
    /// the same step.
    AddAccount,
    /// "Open Again" on the browser step.
    OpenAgain,
    /// A review switch was flipped.
    Toggle(CapabilityKind, Toggle),
    /// The review's last button.
    Confirm,
    /// One step back.
    Back,
    /// "Try Again" on a failure.
    Retry,
    /// Cancel or Escape: the sheet is closed without an answer.
    Cancel,
    /// The shell copied the link or code on screen.
    Copied,
}

/// What the service hears and what it can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsSheetCommand {
    /// Claim `org.quire.AccountsSheet1` (the daemon sends it once the session is up; the service
    /// is passive until then).
    Serve,
    /// The person did this on the sheet of `handle`.
    Act(SheetHandle, SheetAction),
}

/// One thing that happened to the sheets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetEvent {
    /// Whether the shell serves the sheet changed.
    Link(SheetLink),
    /// accountd opened a sheet.
    Opened(SheetHandle, SheetView),
    /// accountd changed what a sheet shows.
    Updated(SheetHandle, SheetView),
    /// accountd took a sheet down.
    Closed(SheetHandle),
    /// The person acted on a sheet.
    Acted(SheetHandle, SheetAction),
    /// accountd left the bus: every sheet it opened has no one to answer.
    Gone,
}

/// What the shell must do after a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetOut {
    /// Tell accountd this about the sheet of the handle (the `Input` signal, to accountd alone).
    Reply(SheetHandle, SheetInput),
    /// Open this page in the person's browser: the sheet was shown a browser step, and showing the
    /// same page again (accountd's answer to "Open Again") opens it again. The host opens a page
    /// it is shown; accountd opens none.
    OpenPage(WebUrl),
}

/// Whether a sheet may be opened or updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    /// Yes.
    Yes,
    /// A sheet of that handle is already open.
    Duplicate,
    /// There are too many open.
    Full,
    /// No sheet of that handle is open.
    Unknown,
}

/// Whether `Open` of `handle` is taken: not twice, and not without limit.
pub fn admits(sheets: &AccountsSheets, handle: &SheetHandle) -> Admit {
    if sheets.open.iter().any(|sheet| &sheet.handle == handle) {
        Admit::Duplicate
    } else if sheets.open.len() >= MAX_OPEN {
        Admit::Full
    } else {
        Admit::Yes
    }
}
