//! The panel's render (design/13 §13.3.9, design/20 §1.3): a `Ds` root on the active Space's
//! look in the `launcher.material` material, holding quire's `CommandPalette` embedded in the
//! surface (no scrim; the card fills the panel and is the blur region) over the fresh results,
//! grouped, and the selected row's actions as a quire `Menu{Rich}` hung from that row
//! (Ctrl+K / Alt+K). A palette mounts with each opening and unmounts when the launcher hides.
//! The palette's selection is the session's; Tab, Shift+Tab, Ctrl+K and Alt+K come from its
//! `onkey`.

use dioxus::prelude::*;
use ds::{
    Anchor, BlurState, CommandPalette, CommandPaletteHost, Ds, Material, Menu, MenuKind,
    PaletteEntrance, Point, Px, Rect, SpaceLook,
};
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
use crate::surfaces::launcher::layer::{CARD_ID, PANEL_HEIGHT, PANEL_WIDTH};
use crate::surfaces::launcher::script::use_script;
use crate::surfaces::launcher::session::Session;

/// The launcher's stylesheet (lint-clean under `Profile::Strict`, `tests/launcher_lint.rs`).
pub(crate) const LAUNCHER_CSS: &str = include_str!("../../style/launcher.css");

/// Where the actions menu hangs while the palette has reported no usable rect: under the first
/// row (its rect in the embedded palette is 6, 80.5, 668 x 45.1; the menu hangs 6 px below).
/// quire reads a new palette's first row before the freshly mapped surface has laid it out and
/// reports 0 x 0, then nothing more until the selection moves (FINDINGS F164), and the selection
/// is always the first row then.
pub const FIRST_ROW_FOOT: Point = Point {
    x: Px(6.0),
    y: Px(125.6),
};

/// A reported row rect, unless it is the empty one quire reports before layout.
pub fn measured(rect: Rect) -> Option<Rect> {
    (rect.size.width.0 > 0.0 && rect.size.height.0 > 0.0).then_some(rect)
}

/// What the palette needs to draw.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shown {
    pub session: Session,
    pub groups: Vec<Group>,
}

impl Shown {
    /// The selected row.
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
    let shown = panel.opening_tracked().map(|opening| {
        let session = panel.session.read().for_opening(&opening);
        Shown {
            groups: shown_groups(&results.read(), &session),
            session,
        }
    });
    rsx! {
        div { id: "panel", class: "sill-launcher", onkeydown: move |event| PanelCtx::root_key(&event),
            Ds {
                appearance,
                system: environment.system,
                look,
                material: material(file.launcher.material),
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                style { {LAUNCHER_CSS} }
                div {
                    class: "sill-launcher-body",
                    style: "width:{PANEL_WIDTH}px;height:{PANEL_HEIGHT}px",
                    if let Some(shown) = shown {
                        {palette(&panel, &shown)}
                    }
                }
            }
        }
    }
}

/// The palette, keyed by its opening so each opening mounts one, and the actions menu while
/// it is open.
fn palette(panel: &PanelCtx, shown: &Shown) -> Element {
    let session = &shown.session;
    let actions = match session.keys.actions {
        ActionsMenu::Open { .. } => shown.selected().map(|item| action_entries(&item.actions)),
        ActionsMenu::Closed => None,
    };
    let (input, pick, close) = (panel.clone(), panel.clone(), panel.clone());
    let (select, key) = (panel.clone(), panel.clone());
    let (action_pick, action_close) = (panel.clone(), panel.clone());
    let mut row = panel.row;
    let selected = shown.selected().map(RowRef::of);
    let hung = (row)();
    rsx! {
        CommandPalette::<RowRef> {
            key: "{session.serial.0}",
            label: "Search",
            placeholder: "Search apps, windows and settings",
            query: session.query.clone(),
            tokens: Vec::new(),
            groups: palette_groups(&shown.groups),
            empty: empty_text(&session.query),
            oninput: move |text: String| input.typed(text),
            onpick: move |row: RowRef| pick.pick(row, sill_launcher::Choice::Primary),
            onclose: move |()| close.dismissed(),
            host: CommandPaletteHost::Surface,
            entrance: PaletteEntrance::CmdkIn,
            id: CARD_ID.to_owned(),
            focus: panel.field,
            selected: session.keys.selected,
            on_select: move |index: usize| select.select(index),
            on_select_rect: move |rect: Rect| row.set(measured(rect)),
            onkey: move |data: KeyboardData| key.field_key(&data),
        }
        if let (Some(entries), Some(row)) = (actions, selected) {
            Menu::<ActionId> {
                kind: MenuKind::Rich,
                anchor: hung.map_or(Anchor::Point(FIRST_ROW_FOOT), Anchor::Rect),
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
    use super::{empty_text, material, measured};
    use ds::{Material, Point, Px, Rect, Size};
    use sill_settings::PanelMaterial;

    #[test]
    fn words_and_material() {
        assert_eq!(empty_text("zz"), "Nothing matches \"zz\".");
        assert_eq!(empty_text(""), "Nothing to show yet.");
        assert_eq!(material(PanelMaterial::Sheet), Material::Sheet);
        assert_eq!(material(PanelMaterial::Popover), Material::Popover);
    }

    #[test]
    fn an_empty_row_rect_is_not_a_place_to_hang_from() {
        let at = |x: f32, y: f32, w: f32, h: f32| Rect {
            origin: Point { x: Px(x), y: Px(y) },
            size: Size {
                width: Px(w),
                height: Px(h),
            },
        };
        assert_eq!(measured(at(0.0, 0.0, 0.0, 0.0)), None, "read before layout");
        let row = at(6.0, 80.5, 668.0, 45.1);
        assert_eq!(measured(row), Some(row));
    }
}
