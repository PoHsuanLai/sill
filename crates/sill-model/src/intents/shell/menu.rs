//! `shell.menu.list` and `shell.menu.activate`: the shell's two actions on the focused window's
//! menus. They are not [`ShellAction`](super::ShellAction)s, which the daemon does by an IPC
//! request: the menu service answers them, one needing the menu's text back, the other a route
//! the menu's table chooses (`app_menu::route_of`).

use docket_core::{AppRefusal, Invocation};

use super::invocation::{param, text};

/// One ask of the menus, its parameters read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAsk {
    /// `shell.menu.list`: the menus of `window` (the focused one when `None`).
    List { window: Option<String> },
    /// `shell.menu.activate`: the item with key `item` of `window`'s menus.
    Activate {
        window: Option<String>,
        item: String,
    },
}

/// The menu ask `invocation` is, `None` when it is another action of the shell.
pub fn menu_ask_of(invocation: &Invocation) -> Result<Option<MenuAsk>, AppRefusal> {
    let window = || text(invocation, "window");
    match invocation.action.as_str() {
        "shell.menu.list" => Ok(Some(MenuAsk::List { window: window()? })),
        "shell.menu.activate" => {
            let item = text(invocation, "item")?
                .map(|item| item.trim().to_owned())
                .filter(|item| !item.is_empty());
            match item {
                Some(item) => Ok(Some(MenuAsk::Activate {
                    window: window()?,
                    item,
                })),
                None => Err(AppRefusal::NeedsParam {
                    param: param("item")?,
                    options: Vec::new(),
                }),
            }
        }
        _ => Ok(None),
    }
}
