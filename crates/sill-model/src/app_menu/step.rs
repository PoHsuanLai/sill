//! The published menus' one transition.

use super::{AppMenuEvent, AppMenus};

/// `menus` after `event`.
pub fn step(menus: AppMenus, event: AppMenuEvent) -> AppMenus {
    let AppMenus { mut served } = menus;
    match event {
        AppMenuEvent::Served { app, menu } => {
            served.insert(app, menu);
        }
        AppMenuEvent::Gone(app) => {
            served.remove(&app);
        }
    }
    AppMenus { served }
}
