//! Next Input Source (the Mac's Control-Space): which input method comes next, and how the last
//! switch went.

mod command;
mod model;
mod next;
#[cfg(test)]
mod tests;

pub use command::InputCommand;
pub use model::{InputFramework, InputRefusal, InputSwitch};
pub use next::next_of;
