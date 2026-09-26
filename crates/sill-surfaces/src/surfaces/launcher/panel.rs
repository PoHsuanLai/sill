//! The panel's render (design/13 §13.3.9, design/20 §1.3): a `Ds` root on the active Space's
//! look in the `launcher.material` material, holding quire's `CommandPalette` embedded in the
//! surface (no scrim; the card is as tall as its content, its corner a squircle, and it is the
//! blur and input region; `card.rs` measures it) over the fresh results in the runner's
//! sections (M9, F645), the selected row's actions as a quire `Menu{Rich}` hung from that row
//! (Ctrl+K / Alt+K), and the preview pane beside the results while it is shown (F654). The box
//! around the root carries the shell type scale (the 22/500 query, 14 and 12 px rows), and the
//! root hands the card the material stack. A palette mounts with each opening and unmounts when
//! the launcher hides. The palette's selection is the session's; Tab, Shift+Tab, Ctrl+K, Alt+K,
//! the pane's keys and the row chords come from its `onkey`.

use dioxus::prelude::*;
use ds::{
    Anchor, BlurState, CommandPalette, CommandPaletteHost, Corner, Ds, Material, Menu, MenuKind,
    PaletteEntrance, Point, Px, Rect, SpaceLook,
};
use shell_host::OutputInfo;
use sill_launcher::{ActionId, Item, Preview, preview_for};
use sill_services::{Activity, OutputName, Workspaces};
use sill_settings::{PanelMaterial, SpacesSettings};

use crate::app_icons::{IconContext, IconLook, icon_look, use_icon_resolver};
use crate::hooks::{
    use_appearance, use_apps, use_results, use_settings, use_space_look, use_workspaces,
};
use crate::surfaces::bar::shell_style::shell_metrics;
use crate::surfaces::bar::workspaces::group_for;
use crate::surfaces::launcher::card::{CardKey, Foot, use_card_hit};
use crate::surfaces::launcher::entries::{RowRef, action_entries};
use crate::surfaces::launcher::events::{PanelCtx, use_panel};
use crate::surfaces::launcher::hit::ActionsShown;
use crate::surfaces::launcher::keys::ActionsMenu;
use crate::surfaces::launcher::layer::{
    CARD_ID, CARD_RADIUS, PANEL_HEIGHT, PANEL_WIDTH, SHADOW_SIDE,
};
use crate::surfaces::launcher::pane_keys::use_link_keys;
use crate::surfaces::launcher::pane_session::PaneSession;
use crate::surfaces::launcher::presence::LauncherLink;
use crate::surfaces::launcher::preview::keys::{Pane, PaneFocus};
use crate::surfaces::launcher::preview::moment::{pane_moment, use_pane_moment};
use crate::surfaces::launcher::preview::{PREVIEW_WIDTH_PX, PaneHost, PreviewPane};
use crate::surfaces::launcher::row_icon::{row_art, row_tile};
use crate::surfaces::launcher::script::use_script;
use crate::surfaces::launcher::sections_view::palette_sections;
use crate::surfaces::launcher::session::Session;
use crate::surfaces::launcher::view::View;

/// The launcher's stylesheet (lint-clean under `Profile::Strict`, `tests/launcher_lint.rs`).
pub(crate) const LAUNCHER_CSS: &str = include_str!("../../style/launcher.css");

/// Where the actions menu hangs while the palette has reported no usable rect: under the first
/// row (its rect in the embedded palette is 6, 80.5, 668 x 45.1 in the card, which starts
/// `SHADOW_SIDE` in; the menu hangs 6 px below).
/// quire reads a new palette's first row before the freshly mapped surface has laid it out and
/// reports 0 x 0, then nothing more until the selection moves (FINDINGS F164), and the selection
/// is always the first row then.
pub const FIRST_ROW_FOOT: Point = Point {
    x: Px(48.0),
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
    pub pane: PaneSession,
    pub view: View,
}

impl Shown {
    /// The selected row (none on a "Show More" line).
    pub fn selected(&self) -> Option<Item> {
        self.view.row(self.session.keys.selected)
    }

    /// What the pane shows: nothing while hidden.
    pub fn preview(&self) -> Option<Preview> {
        match self.pane.preview.pane {
            Pane::Hidden => None,
            Pane::Shown => Some(self.selected().as_ref().map_or(Preview::None, preview_for)),
        }
    }
}

/// The panel on `output`.
pub(crate) fn launcher(output: OutputInfo) -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let appearance = environment.settings.appearance.appearance();
    let icons = RowIcons {
        look: icon_look(&environment.settings.icons, &look, appearance.accent),
        scale: ds_scale.unwrap_or(ds::Scale::ONE),
        index: use_apps()(),
        shipped: file.dock.shipped_icons,
        resolver: use_icon_resolver(),
    };
    let results = use_results();
    let panel = use_panel(output.id, results);
    use_script(panel.clone());
    use_link_keys(panel.clone());
    let chosen = panel.clone();
    use_context_provider(|| PaneHost {
        material: material(file.launcher.material),
        onaction: Callback::new(move |index: usize| chosen.run_pane_action(index)),
    });
    let shown = panel.opening_tracked().map(|opening| {
        let session = panel.session.read().for_opening(&opening);
        let pane = panel.pane.read().for_serial(opening.serial);
        results.read();
        Shown {
            view: panel.view_of(&session, &pane.expanded),
            session,
            pane,
        }
    });
    use_moments(shown.as_ref().and_then(Shown::preview));
    if let Some(shown) = &shown {
        publish_rows(&panel.link, shown);
    }
    let link = use_context::<LauncherLink>();
    let foot = use_card_hit(&link, card_key(shown.as_ref()));
    rsx! {
        div {
            id: "panel",
            class: "sill-launcher",
            style: shell_metrics(&file).style_attr(),
            onkeydown: move |event| PanelCtx::root_key(&event),
            Ds {
                scale: ds_scale,
                appearance,
                system: environment.system,
                look,
                material: material(file.launcher.material),
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                stack: Some(environment.material_stack()),
                style { {LAUNCHER_CSS} }
                div {
                    class: "sill-launcher-body",
                    style: "margin-left:{SHADOW_SIDE}px;width:{PANEL_WIDTH}px;height:{PANEL_HEIGHT}px",
                    if let Some(shown) = shown {
                        {palette(&panel, &shown, foot, &icons)}
                    }
                }
            }
        }
    }
}

/// The pane's moments (D0, F655): it appears, swaps rows, leaves.
fn use_moments(now: Option<Preview>) {
    let mut before = use_hook(|| CopyValue::new(None::<Preview>));
    let moment = pane_moment(before.peek().as_ref(), now.as_ref());
    before.set(now);
    use_pane_moment(moment);
}

/// Tell `sill debug launcher-rows` what shows.
fn publish_rows(link: &LauncherLink, shown: &Shown) {
    let mut rows = shown
        .view
        .report(&shown.session.query, shown.session.keys.selected);
    rows.preview = shown.preview().map(|preview| preview.kind());
    link.show_rows(Some(rows));
}

/// What the card's height follows while it shows.
fn card_key(shown: Option<&Shown>) -> CardKey {
    match shown {
        None => CardKey::Hidden,
        Some(shown) => CardKey::Shown {
            serial: shown.session.serial,
            query: shown.session.query.clone(),
            rows: shown.view.len(),
            // The pane stands beside the card: while it shows, the whole panel takes presses,
            // as it does under the actions menu (the stand-in pane is outside the card's
            // region until quire's `CommandPalette { aside }`, Q292).
            actions: match (shown.session.keys.actions, shown.pane.preview.pane) {
                (ActionsMenu::Open { .. }, _) | (_, Pane::Shown) => ActionsShown::Open,
                (ActionsMenu::Closed, Pane::Hidden) => ActionsShown::Closed,
            },
        },
    }
}

/// The palette, keyed by its opening so each opening mounts one, the preview pane beside it
/// while shown, and the actions menu while it is open.
fn palette(panel: &PanelCtx, shown: &Shown, foot: Foot, icons: &RowIcons) -> Element {
    let session = &shown.session;
    let selected_item = shown.selected();
    let actions = match session.keys.actions {
        ActionsMenu::Open { .. } => selected_item
            .as_ref()
            .map(|item| action_entries(&item.actions)),
        ActionsMenu::Closed => None,
    };
    let (input, pick, close) = (panel.clone(), panel.clone(), panel.clone());
    let (select, key) = (panel.clone(), panel.clone());
    let (action_pick, action_close) = (panel.clone(), panel.clone());
    let mut row = panel.row;
    let selected = selected_item.as_ref().map(RowRef::of);
    let hung = (row)();
    let preview = shown.preview();
    let results_width = match preview {
        Some(_) => PANEL_WIDTH.saturating_sub(PREVIEW_WIDTH_PX),
        None => PANEL_WIDTH,
    };
    let focused = match shown.pane.preview.focus {
        PaneFocus::Pane(index) => Some(index),
        PaneFocus::Results => None,
    };
    let pane_actions = selected_item.map(|item| item.actions).unwrap_or_default();
    rsx! {
        div { class: "sill-launcher-split",
            div {
                class: "sill-launcher-results",
                style: "width:{results_width}px;height:{PANEL_HEIGHT}px",
                CommandPalette::<RowRef> {
                    key: "{session.serial.0}",
                    label: "Search",
                    placeholder: "Search apps, files, windows and settings",
                    query: session.query.clone(),
                    tokens: Vec::new(),
                    groups: palette_sections(&shown.view.sections, &|item| icons.tile(item)),
                    empty: empty_text(&session.query),
                    oninput: move |text: String| input.typed(text),
                    onpick: move |row: RowRef| pick.pick(row, sill_launcher::Choice::Primary),
                    onclose: move |()| close.dismissed(),
                    host: CommandPaletteHost::Surface,
                    entrance: PaletteEntrance::CmdkIn,
                    id: CARD_ID.to_owned(),
                    corner: Some(Corner::Squircle(Px(CARD_RADIUS))),
                    focus: panel.field,
                    selected: session.keys.selected,
                    on_select: move |index: usize| select.select(index),
                    on_select_rect: move |rect: Rect| row.set(measured(rect)),
                    onkey: move |event: KeyboardEvent| key.field_key(&event),
                }
                div {
                    class: "sill-launcher-foot",
                    onmounted: move |event| foot.on_mounted(event),
                }
            }
            if let Some(preview) = preview {
                PreviewPane { preview, actions: pane_actions, focused }
            }
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

/// What the rows' icons are drawn with: the icon style on the active Space, the surface's
/// scale, the app index (which entry a row stands for) and the icon cache.
struct RowIcons {
    look: IconLook,
    scale: ds::Scale,
    index: sill_services::AppIndex,
    shipped: sill_settings::ShippedIcons,
    resolver: crate::app_icons::IconResolver,
}

impl RowIcons {
    fn tile(&self, item: &sill_launcher::Item) -> ds::Tile {
        let cx = IconContext {
            look: self.look,
            scale: self.scale,
            resolver: self.resolver,
        };
        row_tile(item, row_art(item, &self.index, self.shipped), &cx)
    }
}

/// The empty state (design/06 §11.3's "nothing matches", in the ds Menu's words).
pub fn empty_text(query: &str) -> String {
    if query.trim().is_empty() {
        "Nothing to show yet.".to_owned()
    } else {
        format!("Nothing matches \"{}\".", query.trim())
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
/// there is none. The OSD tints the same way.
pub(crate) fn active_look(
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
    let (key, index) = active.unwrap_or((sill_services::WorkspaceKey(0), 0));
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
        assert_eq!(empty_text("zz "), "Nothing matches \"zz\".");
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
