//! The popup root of a context menu a layer surface opens at its pointer: one [`MenuPopup`] of
//! the owner's [`MenuModel`], reporting a pick, a dismissal and its own drop back to the owner
//! over a channel, and closing itself once the menu has closed. The opener hands it a
//! [`ContextMenuLink`] as a root context and [`PopupRoom::CONTEXT`] is its room. A popup is a
//! `VirtualDom` of its own, so it provides quire's document host itself.

use dioxus::prelude::*;
use shell_host::use_surface;
use tokio::sync::mpsc;

use super::{MenuModel, MenuPopup, PopupLook, PopupRoom};

/// What a context menu popup tells its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuReport<Pick> {
    /// An entry was picked.
    Picked(Pick),
    /// The menu closed with nothing picked.
    Dismissed,
    /// The popup's document went away (closed by the compositor, or after a report).
    Gone,
}

/// The popup root's context: the menu, how it looks and where it reports.
#[derive(Clone)]
pub struct ContextMenuLink<Model: MenuModel> {
    pub model: Model,
    pub look: PopupLook,
    /// The document's `data-surface`, for a user stylesheet (`desktop-menu`).
    pub surface: &'static str,
    pub reports: mpsc::UnboundedSender<MenuReport<Model::Pick>>,
}

/// The root of `Model`'s popup (`HostRoot::new(context_menu_popup::<Model>)`).
pub fn context_menu_popup<Model: MenuModel>() -> Element {
    ds_blitz::provide_host();
    let link = use_context::<ContextMenuLink<Model>>();
    let surface = use_surface();
    let gone = link.reports.clone();
    use_drop(move || {
        // The owner may be gone first; then nobody is left to tell.
        let _ = gone.send(MenuReport::Gone);
    });
    let mut picked = use_hook(|| CopyValue::new(false));
    let on_pick = {
        let reports = link.reports.clone();
        move |pick: Model::Pick| {
            picked.set(true);
            let _ = reports.send(MenuReport::Picked(pick));
        }
    };
    // ds `Menu` calls `onpick` and then `onclose`: a close reports a dismissal only when
    // nothing was picked, then closes the popup.
    let on_close = {
        let reports = link.reports.clone();
        let surface = surface.clone();
        move |()| {
            let reports = reports.clone();
            let surface = surface.clone();
            spawn(async move {
                if !picked() {
                    let _ = reports.send(MenuReport::Dismissed);
                }
                surface.close();
            });
        }
    };
    use_context_provider(|| link.look);
    use_context_provider(|| PopupRoom::CONTEXT);
    rsx! {
        MenuPopup {
            model: link.model.clone(),
            on_pick,
            on_close,
            surface: link.surface,
        }
    }
}

impl<Model: MenuModel + std::fmt::Debug> std::fmt::Debug for ContextMenuLink<Model> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextMenuLink")
            .field("model", &self.model)
            .field("surface", &self.surface)
            .finish_non_exhaustive()
    }
}
