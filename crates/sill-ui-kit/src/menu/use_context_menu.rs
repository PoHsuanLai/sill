//! The plumbing both context menus of the shell share (the desktop's and the window menu's):
//! one popup at a time, the channel its reports come back on, and the task that hands each pick
//! to the owner. The owner supplies the menu, where it goes and what a pick does.

use dioxus::prelude::*;
use shell_host::{HostRoot, Rect, SurfaceHandle};
use tokio::sync::mpsc;

use super::{ContextMenuLink, MenuModel, MenuReport, PopupGeom, PopupLook, context_menu_popup};
use crate::settings::SettingsFeed;

/// Whether the surface has its menu's popup up. A second popup may not be created while one holds
/// the grab (xdg-shell: the compositor ends the connection), so a request to open another waits
/// for the first one to be gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Popup {
    Up,
    Down,
}

/// A surface's context menu: what [`use_context_menu`] returns.
pub struct ContextMenu<Model: MenuModel> {
    surface: Option<SurfaceHandle>,
    feed: Option<SettingsFeed>,
    popup: CopyValue<Popup>,
    reports: mpsc::UnboundedSender<MenuReport<Model::Pick>>,
    name: &'static str,
}

// By hand: a derive would ask the menu model itself to be `Clone` and `Debug`.
impl<Model: MenuModel> Clone for ContextMenu<Model> {
    fn clone(&self) -> Self {
        ContextMenu {
            surface: self.surface.clone(),
            feed: self.feed.clone(),
            popup: self.popup,
            reports: self.reports.clone(),
            name: self.name,
        }
    }
}

impl<Model: MenuModel> std::fmt::Debug for ContextMenu<Model> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextMenu")
            .field("name", &self.name)
            .field("popup", &*self.popup.peek())
            .finish_non_exhaustive()
    }
}

impl<Model: MenuModel> ContextMenu<Model> {
    /// Whether no popup of this menu is up, so a request to open one would be shown.
    pub fn is_free(&self) -> bool {
        *self.popup.peek() == Popup::Down
    }

    /// Open `model` drawn in `look`, hung off `anchor` as `geom` says. Nothing opens without a
    /// surface (a test render) or while the menu's last popup is still up.
    pub fn open(&self, model: Model, look: PopupLook, geom: PopupGeom, anchor: Rect) {
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        if !self.is_free() {
            return;
        }
        let mut popup = self.popup;
        popup.set(Popup::Up);
        let mut root = HostRoot::new(context_menu_popup::<Model>).context(ContextMenuLink {
            model,
            look,
            surface: self.name,
            reports: self.reports.clone(),
        });
        if let Some(feed) = self.feed.clone() {
            root = root.context(feed);
        }
        surface.open_popup(geom.config(), geom.placement(anchor), root);
    }
}

/// The context menu of the surface this runs in. `name` is the popup document's `data-surface`
/// (what a user stylesheet names the menu by); `picked` is called with each entry the person
/// picks. A dismissal calls nothing.
pub fn use_context_menu<Model: MenuModel>(
    name: &'static str,
    picked: impl FnMut(Model::Pick) + 'static,
) -> ContextMenu<Model> {
    use_context_menu_closing(name, picked, || {})
}

/// [`use_context_menu`] for an owner that also needs to know when the popup has gone: `closed`
/// is called once per popup, after its last pick, when the menu is dismissed or the popup's
/// document is gone.
pub fn use_context_menu_closing<Model: MenuModel>(
    name: &'static str,
    picked: impl FnMut(Model::Pick) + 'static,
    closed: impl FnMut() + 'static,
) -> ContextMenu<Model> {
    let surface = try_use_context::<SurfaceHandle>();
    let feed = try_use_context::<SettingsFeed>();
    let popup = use_hook(|| CopyValue::new(Popup::Down));
    let (reports, inbox) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<MenuReport<Model::Pick>>();
        (tx, CopyValue::new(Some(rx)))
    });
    use_hook(move || listen(popup, inbox, picked, closed));
    ContextMenu {
        surface,
        feed,
        popup,
        reports,
        name,
    }
}

/// The task that hands each pick the popup reports to `picked`, and notes when the popup is gone.
fn listen<Pick: 'static>(
    popup: CopyValue<Popup>,
    inbox: CopyValue<Option<mpsc::UnboundedReceiver<MenuReport<Pick>>>>,
    mut picked: impl FnMut(Pick) + 'static,
    mut closed: impl FnMut() + 'static,
) {
    let mut inbox = inbox;
    let Some(mut rx) = inbox.write().take() else {
        return;
    };
    let mut popup = popup;
    spawn(async move {
        while let Some(report) = rx.recv().await {
            match report {
                MenuReport::Picked(pick) => picked(pick),
                MenuReport::Dismissed | MenuReport::Gone => {
                    if *popup.peek() == Popup::Up {
                        popup.set(Popup::Down);
                        closed();
                    }
                }
            }
        }
    });
}
