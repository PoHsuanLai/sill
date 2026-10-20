//! The Empty Trash question: the Mac's alert "Are you sure you want to permanently erase the
//! items in the Trash?" with the line "You can't undo this action." and the buttons Cancel (the
//! default, and what Escape presses) and Empty Trash (destructive), centred on the focused
//! window's output over an Overlay layer that takes the keyboard.
//!
//! The daemon opens it on `EmptyTrashLink` when the `EmptyTrash` action is asked; Empty Trash
//! writes a yes on the link and takes the question down, and the daemon empties the Trash. Cancel
//! only takes it down. Like the other sheets this is a layer surface on a stock compositor, and
//! like them it answers nothing for `companion.confirm_arm_ms` after it is drawn.

mod io;
mod surface;
mod view;

pub use surface::EmptyTrashSheet;
