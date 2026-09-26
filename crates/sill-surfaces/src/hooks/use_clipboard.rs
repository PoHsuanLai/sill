//! `use_clipboard`: the clipboard history (M9), for the preview pane's image of a clipboard
//! entry. Its content never reaches a log (`sill_services::clipboard`'s types redact themselves).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Clipboard;

use super::use_services;

/// The clipboard history, newest first.
pub fn use_clipboard() -> ReadSignal<Clipboard> {
    use_watch(use_services().clipboard)
}
