//! Popup menus: the trait an owning surface implements for its menu, the look and room its
//! opener hands the popup, and the popup that draws it.

mod context;
mod frame;
mod look;
mod model;
mod popup;
mod use_context_menu;
#[cfg(test)]
mod tests;

pub use context::{ContextMenuLink, MenuReport, context_menu_popup};
pub use frame::{POPUP_FRAME_CSS, PopupFrame};
pub use look::{CONTEXT_MENU_WIDTH, PopupLook, PopupRoom, use_popup_look};
pub use model::MenuModel;
pub use popup::MenuPopup;
pub use use_context_menu::{ContextMenu, use_context_menu};
