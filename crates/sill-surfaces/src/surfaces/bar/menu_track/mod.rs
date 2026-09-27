//! Menu tracking (design/13 §13.3.2-13.3.4, §13.4 `MenuTrack`, §13.5 safe triangle): click
//! mode, press-drag-release, hover switch between bar menus, submenu delay and the safe
//! triangle. Pure; `now` an argument; effects returned.
//!
//! design/13 §13.7 places this machine in quire (`overlay/menu_track.rs`, driving every ds
//! `Menu`); quire does not have it, so the bar owns it here until it moves (FINDINGS.md, quire
//! gap Q1). Arrow keys inside a menu, wrap, disabled skipping and type-to-filter are the quire
//! `Menu` component's (design/06 §21 decision 3); this machine handles Escape, Left, Right and
//! Enter, which cross menus.

mod machine;
mod triangle;
mod types;

pub use machine::step;
pub use triangle::{guarded, inside};
pub use types::{
    Anim, Branch, Entered, Guard, Held, ItemPath, MenuId, MenuIn, MenuKey, MenuOut, MenuParams,
    MenuTrack, Pick, Shown, Side, Sub, Target,
};

#[cfg(test)]
mod tests;
