//! The machine's effects that leave the dock, as [`DockCommand`]s (design/10 §10.3.6,
//! §10.3.8, §10.3.9): pure over what the services said when the effect came.

use sill_launcher::{AppId, DesktopAction};
use sill_model::apps::AppIndex;
use sill_model::ids::WorkspaceKey;
use sill_model::pins::{DockPins, PinAt, PinEvent};
use sill_model::windows::Toplevels;

use crate::dock::activate::activation;
use crate::dock::items::{DockItem, ItemKind, item_windows};
use crate::dock::machine::{ActivateMode, DockEffect, Instance, TileId};
use crate::dock::pick::{DockCommand, DockPick, pick_commands};

/// The file manager the Trash tile opens `trash:///` in (design/10 §10.3.9).
const FILES_APP: &str = "org.gnome.Nautilus";

/// The desktop action a middle click prefers (design/10 §10.3.6).
const NEW_WINDOW: &str = "new-window";

/// What the effects are decided against.
#[derive(Debug, Clone, Copy)]
pub struct EffectCtx<'a> {
    pub items: &'a [DockItem],
    pub toplevels: &'a Toplevels,
    pub index: &'a AppIndex,
    pub pins: &'a DockPins,
    /// The output's active workspace, when known.
    pub workspace: Option<WorkspaceKey>,
}

/// The commands `effect` sends; effects the dock carries out itself yield none.
pub fn effect_commands(effect: &DockEffect, ctx: &EffectCtx<'_>) -> Vec<DockCommand> {
    let item = |tile: &TileId| ctx.items.iter().find(|i| i.tile == *tile);
    match effect {
        DockEffect::Launch { tile, instance } => item(tile)
            .map(|item| launch(item, *instance, ctx))
            .unwrap_or_default(),
        DockEffect::Activate { tile, mode } => item(tile)
            .map(|item| activate(item, *mode, ctx))
            .unwrap_or_default(),
        DockEffect::SpringOpen { tile } => item(tile)
            .map(|item| activate(item, ActivateMode::Raise, ctx))
            .unwrap_or_default(),
        DockEffect::ShowInFiles { tile } => item(tile)
            .and_then(|item| item.kind.app_id().cloned())
            .map(|app| pick_commands(&DockPick::ShowInFiles(app)))
            .unwrap_or_default(),
        DockEffect::Reorder { tile, to } => item(tile)
            .and_then(|item| reorder(item, *to, ctx.pins))
            .map(DockCommand::Pins)
            .into_iter()
            .collect(),
        DockEffect::Unpin { tile } => item(tile)
            .and_then(|item| match &item.kind {
                ItemKind::Pinned(app) => Some(DockCommand::Pins(PinEvent::Unpin(app.clone()))),
                _ => None,
            })
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn launch(item: &DockItem, instance: Instance, ctx: &EffectCtx<'_>) -> Vec<DockCommand> {
    let Some(app) = item.kind.app_id() else {
        return trash(item);
    };
    let has_new_window = ctx
        .index
        .get(app)
        .is_some_and(|entry| entry.actions.iter().any(|a| a.id == NEW_WINDOW));
    let action = match (instance, has_new_window) {
        (Instance::New, true) => DesktopAction::Named(NEW_WINDOW.to_owned()),
        _ => DesktopAction::Default,
    };
    vec![DockCommand::Launch {
        app: app.clone(),
        action,
        uris: Vec::new(),
    }]
}

/// The Trash tile's launch: the file manager on `trash:///`.
fn trash(item: &DockItem) -> Vec<DockCommand> {
    match item.kind {
        ItemKind::Trash => vec![DockCommand::Launch {
            app: AppId(FILES_APP.to_owned()),
            action: DesktopAction::Default,
            uris: vec!["trash:///".to_owned()],
        }],
        _ => Vec::new(),
    }
}

fn activate(item: &DockItem, mode: ActivateMode, ctx: &EffectCtx<'_>) -> Vec<DockCommand> {
    if item.kind == ItemKind::Trash {
        return trash(item);
    }
    if mode == ActivateMode::Reopen {
        return launch(item, Instance::Existing, ctx);
    }
    let mine = item_windows(&item.kind, ctx.toplevels, ctx.index);
    activation(mode, &mine, ctx.toplevels, ctx.workspace)
        .into_iter()
        .map(DockCommand::Wl)
        .collect()
}

/// A drop at slot `to` among the other app tiles: a pinned tile moves there; an unpinned one
/// dropped before the last pinned tile is pinned there (strictly before, so a short drag of
/// the first running tile in place pins nothing); anything else stays.
fn reorder(item: &DockItem, to: u32, pins: &DockPins) -> Option<PinEvent> {
    let pinned = u32::try_from(pins.pinned.len()).unwrap_or(u32::MAX);
    match &item.kind {
        ItemKind::Pinned(app) => {
            let from = pins.pinned.iter().position(|a| a == app)?;
            let to = to.min(pinned.saturating_sub(1));
            (usize::try_from(to).ok()? != from).then(|| PinEvent::Move {
                app: app.clone(),
                to,
            })
        }
        ItemKind::Running(app) if to < pinned => Some(PinEvent::Pin {
            app: app.clone(),
            at: PinAt::Index(to),
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
