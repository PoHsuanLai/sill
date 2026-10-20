//! The notice board: what the services that tell the person through a notification (accounts,
//! sync hold, guest asks, keycap) share. A service keeps only the words of its notice and how an
//! action key names it; posting, replacing, withdrawing, remembering which id is its own and
//! hearing the server's answer are done here once.

mod board;
mod said;

pub(crate) use board::Board;
pub(crate) use said::{Said, said, stream};
