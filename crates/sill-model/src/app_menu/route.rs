//! What activating a menu item means, and what it costs: the table behind `shell.menu.activate`.
//!
//! Everything an app sends over dbusmenu is a claim. A command id counts only when the app is
//! proven to be the app whose manifest names it (`Identity::Verified`, which the service grants
//! only when porter's caller identity of the name's owner is that app and its menu name is the
//! one quire derives from the app). Then the item is that action, run through intentd so the
//! gate, the confirmation, the undo journal and the audit apply as for a typed call. Every other
//! item of an app is a click the shell cannot weigh, so it is `Destructive` and always asks; the
//! app's own titles ("View", "Window") buy nothing. Only the shell's own rows ([`OwnItem`]) are
//! as cheap as they look.

use std::collections::BTreeMap;

use docket_core::ActionRef;
use prov::{ActionName, AppName, Effect};

use super::OwnItem;
use crate::tray::MenuCommand;

/// An item of the focused app's menus, by the id a listing gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKey {
    /// One of the shell's own rows: `raise:<window>`, `minimise:<window>`, `hide`, `hide-others`, `show-all`, `quit`.
    Own(OwnItem),
    /// A row of the app's menu, by its dbusmenu id.
    Menu(i32),
}

impl ItemKey {
    /// The key written as `text`, or `None` when it names nothing.
    pub fn parse(text: &str) -> Option<ItemKey> {
        let text = text.trim();
        let own = match text {
            "hide" => Some(OwnItem::Hide),
            "hide-others" => Some(OwnItem::HideOthers),
            "show-all" => Some(OwnItem::ShowAll),
            "quit" => Some(OwnItem::Quit),
            _ => window_item(text),
        };
        match own {
            Some(item) => Some(ItemKey::Own(item)),
            None => text.parse().ok().map(ItemKey::Menu),
        }
    }

    /// The key as a listing writes it; [`ItemKey::parse`] reads it back.
    pub fn text(self) -> String {
        match self {
            ItemKey::Own(OwnItem::Raise(key)) => format!("raise:{}", key.0),
            ItemKey::Own(OwnItem::Minimise(key)) => format!("minimise:{}", key.0),
            ItemKey::Own(OwnItem::Hide) => "hide".to_owned(),
            ItemKey::Own(OwnItem::HideOthers) => "hide-others".to_owned(),
            ItemKey::Own(OwnItem::ShowAll) => "show-all".to_owned(),
            ItemKey::Own(OwnItem::Quit) => "quit".to_owned(),
            ItemKey::Menu(id) => id.to_string(),
        }
    }
}

/// `raise:<window>` or `minimise:<window>`.
fn window_item(text: &str) -> Option<OwnItem> {
    let (word, key) = text.split_once(':')?;
    let key = sill_launcher::WindowKey(key.parse().ok()?);
    match word {
        "raise" => Some(OwnItem::Raise(key)),
        "minimise" => Some(OwnItem::Minimise(key)),
        _ => None,
    }
}

/// Whom the owner of a menu's bus name was proven to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// Not proven: the name was claimed, which proves nothing.
    Unverified,
    /// Porter's caller identity names the owner as this app, and the name is the menu name of
    /// that app.
    Verified(AppName),
}

/// The effect of every installed action, from the manifests.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionIndex(pub BTreeMap<ActionRef, Effect>);

/// How an activation is carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// The shell does it from the compositor's state.
    Own(OwnItem),
    /// The verified app's own action, through intentd.
    Intents(ActionRef),
    /// A dbusmenu `Event(id, "clicked")` the shell cannot weigh.
    Click(i32),
}

/// How `item` is activated. `command` is the row's `x-quire-command-id` claim when it has one.
pub fn route_of(
    item: ItemKey,
    command: Option<&MenuCommand>,
    identity: &Identity,
    actions: &ActionIndex,
) -> Route {
    match item {
        ItemKey::Own(own) => Route::Own(own),
        ItemKey::Menu(id) => proven_action(command, identity, actions)
            .map_or(Route::Click(id), Route::Intents),
    }
}

/// The action `command` names in the manifest of the app proven to serve the menu.
fn proven_action(
    command: Option<&MenuCommand>,
    identity: &Identity,
    actions: &ActionIndex,
) -> Option<ActionRef> {
    let (Identity::Verified(app), Some(command)) = (identity, command) else {
        return None;
    };
    let action = ActionRef {
        app: app.clone(),
        name: ActionName::parse(&command.0).ok()?,
    };
    actions.0.contains_key(&action).then_some(action)
}

/// What `route` costs.
pub fn effect_of(route: &Route, actions: &ActionIndex) -> Effect {
    match route {
        Route::Own(own) => own.effect(),
        Route::Intents(action) => actions
            .0
            .get(action)
            .copied()
            .unwrap_or(Effect::Destructive),
        Route::Click(_) => Effect::Destructive,
    }
}
