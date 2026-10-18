//! The panel's render (design/13 §13.3.9, design/20 §1.3): a `Ds` root on the active Space's
//! look in the `launcher.material` material, holding quire's `CommandPalette` embedded in the
//! surface (no scrim; the card is as tall as its content, its corner a squircle, and it is the
//! blur and input region; `card.rs` measures it) over the fresh results in the runner's
//! sections, the selected row's actions as a quire `Menu{Rich}` hung from that row
//! (⌘K), and the preview pane beside the results while it is shown. The box
//! around the root carries the shell type scale (the 22/500 query, 14 and 12 px rows), and the
//! root hands the card the material stack. A palette mounts with each opening and unmounts when
//! the launcher hides. The palette's selection is the session's; Space while browsing, → at the
//! field's end, the pane's keys and the row chords come from its `claim` (before it reads a
//! key), Tab, Shift+Tab and ⌘K from its `onkey` (after).

use dioxus::prelude::*;
use ds::components::menus::palette::palette_claim::FieldKey;
use ds::host::measure::Anchor;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::shape::Corner;
use shell_host::OutputInfo;
use sill_launcher::{ActionId, ClipKey, Item, LauncherMode, Preview, Section, preview_for};
use sill_search::preview::clip_image;

use crate::launcher::card::{CardKey, Foot, use_card_hit};
use crate::launcher::contact::use_contacts;
use crate::launcher::entries::{RowRef, action_entries};
use crate::launcher::events::{PanelCtx, use_panel};
use crate::launcher::hit::ActionsShown;
use crate::launcher::keys::ActionsMenu;
use crate::launcher::layer::{
    CARD_ID, CARD_INSET, CARD_RADIUS, MENU_ID, PANEL_HEIGHT, PANEL_ROOM_WIDTH, PANEL_WIDTH,
    SHADOW_SIDE,
};
use crate::launcher::pane_session::PaneSession;
use crate::launcher::preview::keys::{Pane, PaneFocus};
use crate::launcher::preview::{LauncherPane, PREVIEW_WIDTH_PX};
use crate::launcher::row_icon::{row_art, row_tile};
use crate::launcher::row_thumb::use_row_thumbs;
use crate::launcher::sections_view::{Draw, palette_groups};
use crate::launcher::session::Session;
use crate::launcher::view::View;
use sill_model::voice::listening_line;
use sill_search::ResultsService;
use sill_services::{ClipboardService, VoiceService};
use sill_ui_kit::app_icons::{IconContext, IconLook, icon_look, use_icon_resolver};
use sill_ui_kit::hooks::{use_appearance, use_service, use_service_part, use_settings};
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::look::use_active_look;
use sill_ui_kit::root::{SurfaceRoot, use_host_scale};
use sill_ui_kit::shell_style::{material, shell_metrics};

/// The launcher's stylesheet (lint-clean under `Profile::Strict`, `tests/launcher_lint.rs`).
pub const LAUNCHER_CSS: &str = include_str!("style.css");

/// Where the actions menu hangs while the palette has reported no usable rect: under the first
/// row (its rect in the embedded palette is 6, 80.5, 668 x 38 in the card, which starts
/// `SHADOW_SIDE` and `CARD_INSET` in; the menu hangs 6 px below).
/// quire reads a new palette's first row before the freshly mapped surface has laid it out and
/// reports 0 x 0, then nothing more until the selection moves, and the selection
/// is always the first row then.
pub const FIRST_ROW_FOOT: Point = Point {
    x: Px(48.0 + CARD_INSET as f32),
    y: Px(118.5),
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
    pub mode: LauncherMode,
    /// What voice says on the prompt while the key is held, or after a hold that failed.
    pub voice_line: Option<String>,
}

impl Shown {
    /// The selected row (none on a "Show More" line).
    pub fn selected(&self) -> Option<Item> {
        self.view.row(self.session.keys.selected)
    }

    /// What the pane shows: nothing while hidden. In companion mode an answer's card or a run's
    /// facts is the pane whenever its row is selected (it is the card in the preview pane,
    /// not a pane to open).
    pub fn preview(&self) -> Option<Preview> {
        let selected = self.selected();
        let companions = match (self.mode, &selected) {
            (LauncherMode::Companion, Some(item)) => Some(preview_for(item)).filter(|p| {
                matches!(
                    p,
                    Preview::Answer(_) | Preview::Helper(_) | Preview::NewHelper | Preview::Run(_)
                )
            }),
            _ => None,
        };
        companions.or_else(|| match self.pane.preview.pane {
            Pane::Hidden => None,
            Pane::Shown => Some(selected.as_ref().map_or(Preview::None, preview_for)),
        })
    }
}

/// The panel on `output`.
pub(crate) fn launcher(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let look = use_active_look(&output);
    let appearance = environment.settings.appearance.appearance();
    let icons = RowIcons {
        look: icon_look(
            &environment.settings.icons,
            &look,
            appearance.accent,
            resolve(appearance, look.theme, environment.system).scheme,
        ),
        scale: use_host_scale(),
        resolver: use_icon_resolver(),
    };
    let history = use_service::<ClipboardService>();
    let results = use_service::<ResultsService>();
    let voice_line =
        use_service_part::<VoiceService, _>(|voice| listening_line(&voice.phase, voice.notice));
    let panel = use_panel(output.id, results);
    let root = panel.clone();
    let shown = panel.opening_tracked().map(|opening| {
        let session = panel.session.read().for_opening(&opening);
        let pane = panel.pane.read().for_serial(opening.serial);
        results.read();
        Shown {
            view: panel.view_of(&session, &pane.expanded),
            session,
            pane,
            mode: opening.mode,
            voice_line: voice_line.read().clone(),
        }
    });
    let contacts = use_contacts();
    let touch = contacts.take();
    let aside = use_aside(shown.as_ref(), &icons, panel.left, touch);
    if let Some(shown) = &shown {
        publish_rows(&panel.link, shown);
    }
    let link = use_context::<LauncherLink>();
    let foot = use_card_hit(&link, card_key(shown.as_ref(), aside.as_ref()));
    let (width, inset) = match aside {
        Some(_) => (PANEL_ROOM_WIDTH, 0),
        None => (PANEL_WIDTH, CARD_INSET),
    };
    let left = SHADOW_SIDE + inset;
    let thumbs = use_row_thumbs();
    let thumb_of = move |mime: &str, path: &std::path::Path| thumbs.of(mime, path);
    let clip_image = move |key: ClipKey| {
        clip_image(&history.read(), key)
            .filter(|image| image.mime.0 == "image/png")
            .map(|image| ImageSource::png(&image.bytes))
    };
    rsx! {
        div {
            id: "panel",
            class: "sill-launcher",
            style: shell_metrics(&file).style_attr(),
            onkeydown: move |event| {
                contacts.heard(&event);
                root.root_key(&event);
            },
            onmousedown: move |event| contacts.heard(&event),
            SurfaceRoot {
                output,
                material: material(file.launcher.material),
                AppStyle { css: LAUNCHER_CSS }
                div {
                    class: "sill-launcher-body",
                    style: "margin-left:{left}px;width:{width}px;height:{PANEL_HEIGHT}px",
                    if let Some(shown) = shown {
                        {palette(&panel, &shown, foot, &icons, Images { thumb_of: &thumb_of, clip_image: &clip_image }, Beside { aside })}
                    }
                }
            }
        }
    }
}

/// The pane as the palette's `aside` draws it: what it previews, the row's actions and icon,
/// the control with the keyboard, and whether it is showing or leaving.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Aside {
    preview: Preview,
    actions: Vec<sill_launcher::Action>,
    icon: Option<IconSource>,
    focused: Option<usize>,
    shown: ds::prelude::Shown,
    serial: sill_ui_kit::links::OpenSerial,
    touch: ds::motion::detail::touch::Touch,
}

/// The pane while it shows, and after it is hidden until quire has played it out
/// (`on_hidden`), so it leaves as it came.
fn use_aside(
    shown: Option<&Shown>,
    icons: &RowIcons,
    left: Signal<u64>,
    touch: ds::motion::detail::touch::Touch,
) -> Option<Aside> {
    let mut last = use_hook(|| CopyValue::new(None::<Aside>));
    let mut seen = use_hook(|| CopyValue::new(0_u64));
    let count = *left.read();
    if count != *seen.peek() {
        seen.set(count);
        last.set(None);
    }
    let Some(shown) = shown else {
        last.set(None);
        return None;
    };
    match shown.preview() {
        Some(preview) => {
            let selected = shown.selected();
            let aside = Aside {
                preview,
                icon: selected
                    .as_ref()
                    .and_then(|item| icon_source(icons.tile(item))),
                actions: selected.map(|item| item.actions).unwrap_or_default(),
                focused: match shown.pane.preview.focus {
                    PaneFocus::Pane(index) => Some(index),
                    PaneFocus::Results => None,
                },
                shown: ds::prelude::Shown::Visible,
                serial: shown.session.serial,
                touch,
            };
            last.set(Some(aside.clone()));
            Some(aside)
        }
        None => last
            .peek()
            .clone()
            .filter(|leaving| leaving.serial == shown.session.serial)
            .map(|leaving| Aside {
                shown: ds::prelude::Shown::Hidden,
                focused: None,
                touch,
                ..leaving
            }),
    }
}

/// A row's tile as the pane's icon.
fn icon_source(tile: RowLeading) -> Option<IconSource> {
    match tile {
        RowLeading::Source(source) => Some(source),
        RowLeading::Icon(icon) => Some(IconSource::Glyph(icon)),
        _ => None,
    }
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
fn card_key(shown: Option<&Shown>, aside: Option<&Aside>) -> CardKey {
    match shown {
        None => CardKey::Hidden,
        Some(shown) => CardKey::Shown {
            serial: shown.session.serial,
            query: shown.session.query.clone(),
            rows: shown.view.len(),
            // While the pane widens the card, the whole room takes presses, as it does under
            // the actions menu: the catcher's hole is the room then (`catcher::layer::input`).
            actions: match (shown.session.keys.actions, aside) {
                (ActionsMenu::Open { .. }, _) | (_, Some(_)) => ActionsShown::Open,
                (ActionsMenu::Closed, None) => ActionsShown::Closed,
            },
        },
    }
}

/// Where the palette's rows get their pictures: a file's thumbnail, a clipboard item's image.
struct Images<'a> {
    thumb_of: &'a dyn Fn(&str, &std::path::Path) -> Option<ImageSource>,
    clip_image: &'a dyn Fn(ClipKey) -> Option<ImageSource>,
}

/// The palette, keyed by its opening so each opening mounts one, the preview pane as its
/// `aside` while shown or leaving, and the actions menu while it is open.
fn palette(
    panel: &PanelCtx,
    shown: &Shown,
    foot: Foot,
    icons: &RowIcons,
    images: Images<'_>,
    beside: Beside,
) -> Element {
    let Images {
        thumb_of,
        clip_image,
    } = images;
    let Beside { aside } = beside;
    let session = &shown.session;
    let selected_item = shown.selected();
    let actions = match session.keys.actions {
        ActionsMenu::Open { .. } => selected_item
            .as_ref()
            .map(|item| action_entries(&item.actions)),
        ActionsMenu::Closed => None,
    };
    let (input, pick, close) = (panel.clone(), panel.clone(), panel.clone());
    let (select, key, claim) = (panel.clone(), panel.clone(), panel.clone());
    let (action_pick, action_close) = (panel.clone(), panel.clone());
    let mut row = panel.row;
    let selected = selected_item.as_ref().map(RowRef::of);
    let hung = (row)();
    let draw = Draw {
        tile_of: &|item| icons.tile(item),
        thumb_of,
        clip_image,
        now: now(),
    };
    let on_more = |section: Section| {
        let panel = panel.clone();
        EventHandler::new(move |()| panel.toggle_section(section))
    };
    let groups = palette_groups(&shown.view.sections, &draw, &on_more);
    let pane = aside.map(|aside| pane(panel, aside));
    rsx! {
        CommandPalette::<RowRef> {
            key: "{session.serial.0}",
            label: match shown.mode {
                LauncherMode::Search => "Search",
                LauncherMode::Companion => "Ask",
            },
            placeholder: match shown.mode {
                LauncherMode::Search => "Search apps, files, windows and settings".to_owned(),
                LauncherMode::Companion => shown
                    .voice_line
                    .clone()
                    .unwrap_or_else(|| "Ask the assistant".to_owned()),
            },
            query: session.query.clone(),
            tokens: Vec::new(),
            groups,
            empty: empty_text(&session.query),
            oninput: move |text: String| input.typed(text),
            onpick: move |row: RowRef| pick.pick(row, sill_launcher::Choice::Primary),
            onclose: move |()| close.dismissed(),
            host: CommandPaletteHost::Surface,
            id: CARD_ID.to_owned(),
            corner: Some(Corner::Squircle(Px(CARD_RADIUS))),
            focus: panel.field,
            selected: session.keys.selected,
            on_select: move |index: usize| select.select(index),
            on_select_rect: move |rect: Rect| row.set(measured(rect)),
            onkey: move |event: KeyboardEvent| key.field_key(&event),
            claim: Callback::new(move |field: FieldKey| claim.claim(&field)),
            handle: Some(panel.palette),
            aside: pane,
            aside_width: Px(PREVIEW_WIDTH_PX as f32),
        }
        div {
            class: "sill-launcher-foot",
            onmounted: move |event| foot.on_mounted(event),
        }
        if let (Some(entries), Some(row)) = (actions, selected) {
            Menu::<ActionId> {
                placement: MenuPlacement::Popup,
                common: Common { id: Some(MENU_ID.to_owned()), ..Common::default() },
                anchor: hung.map_or(Anchor::Point(FIRST_ROW_FOOT), Anchor::Rect),
                items: entries,
                // The first action starts highlighted, so Return runs it at once.
                start: Some(0),
                onpick: move |action: ActionId| {
                    action_pick.pick(row.clone(), sill_launcher::Choice::Action(action))
                },
                onclose: move |()| action_close.actions_closed(),
            }
        }
    }
}

/// What the palette plays beside its rows: the preview pane.
struct Beside {
    aside: Option<Aside>,
}

/// The preview pane for `aside`: quire's, through [`LauncherPane`].
fn pane(panel: &PanelCtx, aside: Aside) -> Element {
    let (act, gone) = (panel.clone(), panel.clone());
    rsx! {
        LauncherPane {
            preview: aside.preview,
            actions: aside.actions,
            focused: aside.focused,
            icon: aside.icon,
            shown: aside.shown,
            touch: aside.touch,
            onaction: move |index: usize| act.run_pane_action(index),
            on_hidden: move |()| gone.pane_gone(),
        }
    }
}

fn now() -> sill_launcher::UnixTime {
    sill_launcher::UnixTime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}

/// What the rows' icons are drawn with: the icon style on the active Space, the surface's
/// scale, and the icon cache.
struct RowIcons {
    look: IconLook,
    scale: Scale,
    resolver: sill_ui_kit::app_icons::IconResolver,
}

impl RowIcons {
    fn tile(&self, item: &sill_launcher::Item) -> RowLeading {
        let cx = IconContext {
            look: self.look,
            scale: self.scale,
            resolver: self.resolver,
        };
        row_tile(item, row_art(item), &cx)
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

#[cfg(test)]
mod tests {
    use super::{empty_text, measured};
    use ds::prelude::*;
    use sill_settings::PanelMaterial;
    use sill_ui_kit::shell_style::material;

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
