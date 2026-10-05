//! Activating an item for an agent.

use std::collections::BTreeMap;

use docket_client::Transport;
use docket_core::{ActionRef, CallRequest, Origin, TargetValue};
use prov::Effect;
use sill_model::app_menu::{
    ActionIndex, Identity, ItemKey, Route, effect_of, own_commands, route_of,
};
use sill_model::tray::{MenuCommand, TrayMenuEntry, TrayMenuKind};
use sill_model::units::Availability;

use super::MenuAgent;
use crate::app_menu::wire;

/// Why an ask about a menu did not happen. The words never carry the app's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuFault {
    /// No such window.
    NoWindow,
    /// The window's app serves no menu.
    NoMenu,
    /// No item has that key.
    NoSuchItem,
    /// The item is disabled, or opens a submenu and runs nothing.
    NotRunnable,
    /// intentd refused the verified app's own action.
    Refused(String),
    /// Something else failed.
    Failed(String),
}

/// What an activation did and what the table said it costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activation {
    pub how: Activated,
    /// The effect of the route taken (`sill_model::app_menu::effect_of`).
    pub effect: Effect,
}

/// How an activation was carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activated {
    /// The shell did one of its own rows.
    Own,
    /// The verified app's action ran through intentd.
    Action(ActionRef),
    /// The app was sent a click it must weigh itself.
    Clicked,
}

impl<T: Transport> MenuAgent<T> {
    /// Activate `item` of the window asked for (the focused one when `window` is `None`). `origin`
    /// is where the call came from, which an action run through intentd keeps.
    pub async fn activate(
        &self,
        window: Option<&str>,
        item: &str,
        origin: Origin,
    ) -> Result<Activation, MenuFault> {
        let key = ItemKey::parse(item).ok_or(MenuFault::NoSuchItem)?;
        let target = self.target(window)?;
        let id = match key {
            ItemKey::Own(own) => {
                for command in own_commands(own, &target.app.0, &target.toplevels) {
                    self.links
                        .wl
                        .send(command)
                        .map_err(|_| MenuFault::Failed("the compositor is not there".to_owned()))?;
                }
                let effect = effect_of(&Route::Own(own), &ActionIndex::default());
                return Ok(Activation {
                    how: Activated::Own,
                    effect,
                });
            }
            ItemKey::Menu(id) => id,
        };
        let served = self.serve(&target.app).await?.ok_or(MenuFault::NoMenu)?;
        let row = find(&served.menu.menus, id).ok_or(MenuFault::NoSuchItem)?;
        let command = runnable(row)?;
        let identity = self.identity(&served).await;
        let actions = self.actions(command, &identity).await?;
        let route = route_of(key, command, &identity, &actions);
        let effect = effect_of(&route, &actions);
        let how = match route {
            Route::Intents(action) => self.perform(action, origin).await?,
            Route::Click(id) => {
                wire::click(&self.connection, served.owner.as_str(), id)
                    .await
                    .map_err(|_| MenuFault::Failed("the app could not be told".to_owned()))?;
                Activated::Clicked
            }
            Route::Own(_) => return Err(MenuFault::NoSuchItem),
        };
        Ok(Activation { how, effect })
    }

    /// The installed actions and their effects; none are read unless a proven app names one.
    async fn actions(
        &self,
        command: Option<&MenuCommand>,
        identity: &Identity,
    ) -> Result<ActionIndex, MenuFault> {
        if command.is_none() || *identity == Identity::Unverified {
            return Ok(ActionIndex::default());
        }
        let manifests = self
            .intents
            .manifests()
            .await
            .map_err(|_| MenuFault::Failed("intentd could not be asked".to_owned()))?;
        let actions: BTreeMap<ActionRef, _> = manifests
            .iter()
            .flat_map(|valid| {
                let manifest = valid.manifest();
                manifest.actions.iter().map(|action| {
                    (
                        ActionRef {
                            app: manifest.app.clone(),
                            name: action.name.clone(),
                        },
                        action.effect,
                    )
                })
            })
            .collect();
        Ok(ActionIndex(actions))
    }

    async fn perform(&self, action: ActionRef, origin: Origin) -> Result<Activated, MenuFault> {
        let call = CallRequest {
            action: action.clone(),
            target: TargetValue::Nothing,
            args: BTreeMap::new(),
            origin,
        };
        match self.intents.perform(call, None, None).await {
            Ok(Ok(_outcome)) => Ok(Activated::Action(action)),
            Ok(Err(refusal)) => Err(MenuFault::Refused(format!("{refusal:?}"))),
            Err(_) => Err(MenuFault::Failed("intentd could not be asked".to_owned())),
        }
    }
}

/// The row with dbusmenu id `id`, anywhere in `rows`.
fn find(rows: &[TrayMenuEntry], id: i32) -> Option<&TrayMenuEntry> {
    rows.iter().find_map(|row| {
        if row.id == id {
            return Some(row);
        }
        match &row.kind {
            TrayMenuKind::Item { children, .. } => find(children, id),
            TrayMenuKind::Separator => None,
        }
    })
}

/// `row`'s command claim, when it is a row that runs something: enabled and no submenu.
fn runnable(row: &TrayMenuEntry) -> Result<Option<&MenuCommand>, MenuFault> {
    match &row.kind {
        TrayMenuKind::Item {
            enabled: Availability::Enabled,
            children,
            command,
            ..
        } if children.is_empty() => Ok(command.as_ref()),
        TrayMenuKind::Item { .. } | TrayMenuKind::Separator => Err(MenuFault::NotRunnable),
    }
}
