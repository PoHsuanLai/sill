//! A window's menu (the one the Mac opens when the pointer rests on a window's green light):
//! one transparent Overlay surface per output that takes no input of its own, and, when the
//! compositor asks for a menu on its output ([`sill_ui_kit::links::WindowMenuLink`]), a context
//! menu popup of quire's `Menu` opened at the point it names. Enter or Exit Full Screen come
//! first, then the Move & Resize and Full Screen Tile submenus, then Fill and Center; the
//! entries the window's state does not offer are dimmed. A pick is written back to the link and
//! the daemon carries it out; a dismissal writes nothing.

mod geometry;
mod io;
mod layer;
mod model;
#[cfg(test)]
mod popup_tests;
mod surface;
mod view;

pub use surface::WindowMenu;
