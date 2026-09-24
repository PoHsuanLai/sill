//! The panel's render (design/13 §13.3.9, design/20 §1.3): a `Ds` root on the active Space's
//! look in the `launcher.material` material, holding quire's `CommandPalette` over the fresh
//! results, grouped, and the selected row's actions as a quire `Menu{Rich}` (Ctrl+K / Alt+K).
//! The keys the palette does not handle (Tab, Shift+Tab, Ctrl+K, Alt+K, and Escape while the
//! actions menu is open) are read on the surface's own root, which every key event reaches.

use dioxus::prelude::*;
use ds::{Anchor, BlurState, CommandPalette, Ds, Material, Menu, MenuKind, Point, Px, SpaceLook};
use shell_host::OutputInfo;
use sill_launcher::{ActionId, Item};
use sill_services::{Activity, OutputName, Results, WorkspaceKey, Workspaces};
use sill_settings::{PanelMaterial, SpacesSettings};

use crate::hooks::{use_appearance, use_results, use_settings, use_space_look, use_workspaces};
use crate::surfaces::bar::workspaces::group_for;
use crate::surfaces::launcher::entries::{RowRef, action_entries, palette_groups};
use crate::surfaces::launcher::events::{PanelCtx, use_panel};
use crate::surfaces::launcher::groups::{Group, fresh, groups, rows};
use crate::surfaces::launcher::keys::ActionsMenu;
use crate::surfaces::launcher::layer::PANEL_HEIGHT;
use crate::surfaces::launcher::script::use_script;
use crate::surfaces::launcher::session::Session;

/// The launcher's stylesheet (lint-clean under `Profile::Strict`, `tests/launcher_lint.rs`).
pub(crate) const LAUNCHER_CSS: &str = include_str!("../../style/launcher.css");

/// Where the actions menu opens: over the palette card's right half, just under its search
/// row (the palette card is 540 wide and 11 % of the panel's width down, command_palette.css;
/// quire's palette reports no row rects to anchor to, Q41).
pub const ACTIONS_AT: Point = Point {
    x: Px(330.0),
    y: Px(131.0),
};

/// What the palette needs to draw.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shown {
    pub session: Session,
    pub groups: Vec<Group>,
    /// The palette's mount (`mounting.rs`).
    pub key: u64,
}

impl Shown {
    /// The selected row, as the keyboard machine tracks it.
    pub fn selected(&self) -> Option<&Item> {
        rows(&self.groups).get(self.session.keys.selected).copied()
    }
}

/// The groups `session` shows over `results`.
pub(crate) fn shown_groups(results: &Results, session: &Session) -> Vec<Group> {
    groups(
        fresh(results, session.since),
        &session.query,
        session.keys.filter,
    )
}

/// The panel on `output`.
pub(crate) fn launcher(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let appearance = environment.settings.appearance.appearance();
    let results = use_results();
    let panel = use_panel(output.id, results);
    use_script(panel.clone());
    // While hidden, the last opening's palette stays mounted (`mounting.rs`).
    let session = match panel.opening_tracked() {
        Some(opening) => panel.session.read().for_opening(&opening),
        None => panel.session.read().clone(),
    };
    let key = panel.palette.read().key;
    let shown = (key > 0).then(|| Shown {
        groups: shown_groups(&results.read(), &session),
        session,
        key,
    });
    let onkey = {
        let panel = panel.clone();
        move |event: KeyboardEvent| panel.key(&event)
    };
    rsx! {
        div { id: "panel", class: "sill-launcher", onkeydown: onkey,
            Ds {
                appearance,
                system: environment.system,
                look,
                material: material(file.launcher.material),
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                style { {LAUNCHER_CSS} }
                div { class: "sill-launcher-body", style: "height:{PANEL_HEIGHT}px" }
                if let Some(shown) = shown {
                    {palette(&panel, &shown)}
                }
            }
        }
    }
}

/// The palette, and the actions menu while it is open.
fn palette(panel: &PanelCtx, shown: &Shown) -> Element {
    let session = &shown.session;
    let actions = match session.keys.actions {
        ActionsMenu::Open { .. } => shown.selected().map(|item| action_entries(&item.actions)),
        ActionsMenu::Closed => None,
    };
    let (input, pick, close) = (panel.clone(), panel.clone(), panel.clone());
    let (action_pick, action_close) = (panel.clone(), panel.clone());
    let selected = shown.selected().map(RowRef::of);
    rsx! {
        CommandPalette::<RowRef> {
            key: "{shown.key}",
            label: "Search",
            placeholder: "Search apps, windows and settings",
            query: session.query.clone(),
            tokens: Vec::new(),
            groups: palette_groups(&shown.groups),
            empty: empty_text(&session.query),
            oninput: move |text: String| input.typed(text),
            onpick: move |row: RowRef| pick.pick(row, sill_launcher::Choice::Primary),
            onclose: move |()| close.dismissed(),
        }
        if let (Some(entries), Some(row)) = (actions, selected) {
            Menu::<ActionId> {
                kind: MenuKind::Rich,
                anchor: Anchor::Point(ACTIONS_AT),
                entries,
                onpick: move |action: ActionId| {
                    action_pick.pick(row.clone(), sill_launcher::Choice::Action(action))
                },
                onclose: move |()| action_close.actions_closed(),
            }
        }
    }
}

/// The empty state (design/06 §11.3's "nothing matches", in the ds Menu's words).
pub fn empty_text(query: &str) -> String {
    if query.trim().is_empty() {
        "Nothing to show yet.".to_owned()
    } else {
        format!("Nothing matches \"{query}\".")
    }
}

/// `launcher.material` in quire's words (design/22 Sheet; design/20 Popover).
pub fn material(material: PanelMaterial) -> Material {
    match material {
        PanelMaterial::Sheet => Material::Sheet,
        PanelMaterial::Popover => Material::Popover,
    }
}

/// The look of the active workspace on `output` (design/21 §5 rule 6), the first preset while
/// there is none.
fn active_look(
    workspaces: &Workspaces,
    output: Option<&OutputName>,
    spaces: &SpacesSettings,
) -> SpaceLook {
    let active = group_for(&workspaces.groups, output).and_then(|group| {
        group
            .workspaces
            .iter()
            .position(|w| w.activity == Activity::Active)
            .map(|index| (group.workspaces[index].key, index))
    });
    let (key, index) = active.unwrap_or((WorkspaceKey(0), 0));
    use_space_look(key, index, spaces)
}

#[cfg(test)]
mod tests {
    use super::{empty_text, material};
    use ds::Material;
    use sill_settings::PanelMaterial;

    #[test]
    fn words_and_material() {
        assert_eq!(empty_text("zz"), "Nothing matches \"zz\".");
        assert_eq!(empty_text(""), "Nothing to show yet.");
        assert_eq!(material(PanelMaterial::Sheet), Material::Sheet);
        assert_eq!(material(PanelMaterial::Popover), Material::Popover);
    }
}
