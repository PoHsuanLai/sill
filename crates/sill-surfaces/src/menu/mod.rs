//! Popup menus: the trait an owning surface implements for its menu, the look and room its
//! opener hands the popup, and the popup that draws it.

mod look;
mod model;
mod popup;
#[cfg(test)]
mod tests;

pub use look::{PopupLook, PopupRoom};
pub use model::MenuModel;
pub use popup::MenuPopup;
