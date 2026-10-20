//! The daemon's half of the Empty Trash question: the `EmptyTrash` action opens it on the link
//! (`actions::panel::empty_trash`), the surface shows it, and a yes empties the home Trash on a
//! thread of its own (`sill_services::trash`). Cancel sends nothing, so nothing is erased.

use sill_services::env::Dirs;
use sill_ui_kit::links::EmptyTrashLink;
use tokio::runtime::Handle;

/// Follow the yes on `link` on `rt` and empty the Trash for each one.
pub fn spawn_follower(rt: &Handle, link: &EmptyTrashLink) {
    let mut yes = link.subscribe_answered();
    rt.spawn(async move {
        while yes.changed().await.is_ok() {
            if yes.borrow_and_update().is_some() {
                sill_services::trash::empty_in_background(Dirs::from_process());
            }
        }
    });
}
