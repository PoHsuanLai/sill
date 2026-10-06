//! The effects end of the sheet: what the person does goes to the service as an `Act` for the
//! sheet on screen, and the link or code copied goes to the clipboard through the surface (then
//! the sheet marks it copied). The drawn parts decide nothing.

use dioxus::prelude::*;
use shell_host::SurfaceHandle;
use sill_model::accounts::sheet::{AccountsSheetCommand, SheetAction, SheetHandle};
use sill_services::accounts::sheet::AccountsSheetService;
use sill_ui_kit::hooks::use_control;

/// What the steps send.
#[derive(Clone, Copy)]
pub(super) struct Io {
    /// The person did this.
    pub act: Callback<SheetAction>,
    /// Copy this text, and say it was copied.
    pub copy: Callback<String>,
}

/// The effects for the sheet of `handle`.
pub(super) fn use_io(handle: SheetHandle) -> Io {
    let sheet = use_control::<AccountsSheetService>();
    let surface = try_use_context::<SurfaceHandle>();
    let act = use_callback(move |action: SheetAction| {
        // A stopped service has no one to tell; the sheet goes with it.
        let _ = sheet.send(AccountsSheetCommand::Act(handle.clone(), action));
    });
    let copy = use_callback(move |text: String| {
        if let Some(surface) = &surface {
            surface.copy_text(text);
            act.call(SheetAction::Copied);
        }
    });
    Io { act, copy }
}
