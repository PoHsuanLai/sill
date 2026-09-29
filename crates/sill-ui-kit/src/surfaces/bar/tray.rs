//! The tray: one `IconButton{Status}` per status notifier item, its dbusmenu as a quire `Menu`.
//!
//! Each item draws its own icon: a pixmap or a theme icon through `sill_services::icon_art`,
//! symbolic (drawn in the frame's ink, like a glyph) or an image by design/08-ICONS.md §1.5, as
//! `tray_input::icon_source` decides with quire's `ds::icon::classify`. The button carries the
//! `tray-<n>` id the popup anchors to, and hands over the pointer button and point: the primary
//! button opens the dbusmenu (or activates an item without one), the secondary opens the
//! dbusmenu, the middle one is SNI's secondary activation, each with the press's screen point.
//! The wheel over an item is SNI `Scroll`. A dbusmenu item's submenu is still dropped (the popup
//! is not sized for one beside the menu); a disabled row is drawn disabled.
//!
//! `TrayButtonView` carries no hook: it is the part `tests` renders directly. `TrayButton`
//! wraps it with the hooks a live surface needs (`use_surface`, the tray `Control`), neither
//! of which a plain unit test can construct (`SurfaceHandle`'s constructor is
//! `pub(crate)` to shell-host, and `Control` has no `PartialEq`, so it cannot be a `#[component]`
//! prop at all: `TrayButton` reads it itself instead of receiving it).

use dioxus::prelude::*;
use ds::{
    Check as DsCheck, ChromaLimit, Common, IconButton, IconButtonVariant, IconSource, MenuEntry,
    MenuTrail, Press,
};
use shell_host::{
    HostRoot, OutputInfo, PopupConfig, PopupPlacement, PopupSize, Rect, Size, SurfaceHandle,
    use_surface,
};
use sill_model::tray::{
    Check as TrayCheck, TrayCommand, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind,
    TrayStatus,
};
use sill_model::units::{Availability as TrayAvailability, Visibility};
use sill_services::tray::icon_art;
use sill_services::{Control, TrayService};

use super::geometry::menu_size;
use super::popup::{MenuSide, menu_popup};
use super::tray_input::{
    ItemMenu, ScreenOrigin, TrayAction, chroma_limit, icon_source, press_action, scroll_delta,
};
use super::tray_menu::{POPUP_WIDTH, TrayMenuDocument};
use crate::hooks::{use_appearance, use_control, use_service, use_settings};

/// The room under the menu's estimated height (its 8 px inset from the popup's edge, with
/// slack for the estimate).
const POPUP_SLACK: i32 = 16;
/// The tallest the popup is made, a guard of ours rather than a design value (no design doc
/// caps a menu's height): a longer dbusmenu is clipped at its foot. Also `menu_popup`'s
/// `max_px`.
const POPUP_MAX: i32 = 640;

/// The element id a popup anchors to for the button at `index` (`tray-<n>`, as the module doc
/// says).
fn popup_element_id(index: usize) -> String {
    format!("tray-{index}")
}

/// The tray: one `IconButton{Status}` per status notifier item, its dbusmenu as a quire
/// `Menu{Dropdown}`. The items are global, not per-output, and each output's bar is its own
/// document, so `tray-<n>` needs no output qualifier; `output` places the presses on screen.
#[component]
pub fn Tray(output: OutputInfo) -> Element {
    let tray = use_service::<TrayService>();
    let items = tray().items;
    let origin = ScreenOrigin {
        x: output.logical.x,
        y: output.logical.y,
    };
    rsx! {
        for (index , item) in items.into_iter().enumerate() {
            TrayButton { key: "{item.id.0}", index, item, origin }
        }
    }
}

/// `icon`'s art as quire draws it at the chroma `limit`, resolved (theme lookup, file read,
/// chroma test) only when the icon or the limit changes, not on every render.
fn use_icon_source(icon: &TrayIcon, size_px: u16, limit: ChromaLimit) -> IconSource {
    let mut cache = use_hook(|| CopyValue::new(None::<(TrayIcon, ChromaLimit, IconSource)>));
    if let Some((seen, seen_limit, source)) = cache.peek().as_ref()
        && seen == icon
        && *seen_limit == limit
    {
        return source.clone();
    }
    let source = icon_source(icon_art(icon, size_px), limit);
    cache.set(Some((icon.clone(), limit, source.clone())));
    source
}

/// One status item, wired to the live surface: a press does what `press_action` says, a picked
/// menu entry sends `MenuPick`, and the wheel sends `Scroll`.
#[component]
fn TrayButton(index: usize, item: TrayItem, origin: ScreenOrigin) -> Element {
    let surface = use_surface();
    let control = use_control::<TrayService>();
    let size_px = use_settings()().bar.status_icon_box_px.0;
    let limit = chroma_limit(use_appearance()().settings.icons.symbolic_chroma_max.0);
    let icon = use_icon_source(&item.icon, size_px, limit);
    let popup_id = popup_element_id(index);
    let menu = item.menu.clone();
    let item_id = item.id.clone();
    let has_menu = if menu.is_empty() {
        ItemMenu::Absent
    } else {
        ItemMenu::Present
    };
    let onclick = {
        let control = control.clone();
        let item_id = item_id.clone();
        move |press: Press| match press_action(press, &item_id, has_menu, origin) {
            TrayAction::Send(command) => {
                let _ = control.send(command);
            }
            TrayAction::OpenMenu => {
                let _ = control.send(TrayCommand::MenuOpening {
                    item: item_id.clone(),
                });
                open_menu(&surface, &popup_id, &item_id, &menu, &control);
            }
            TrayAction::Nothing => {}
        }
    };
    let onwheel = move |event: WheelEvent| {
        if let Some(delta) = scroll_delta(event.delta()) {
            let _ = control.send(TrayCommand::Scroll {
                item: item_id.clone(),
                delta,
            });
        }
    };
    rsx! {
        TrayButtonView { index, item, icon, onclick, onwheel }
    }
}

/// One status item's markup: no hook, so `tests` can render it directly. The unclassed `span`
/// only listens for the wheel, which `IconButton` does not report.
#[component]
fn TrayButtonView(
    index: usize,
    item: TrayItem,
    icon: IconSource,
    onclick: EventHandler<Press>,
    onwheel: EventHandler<WheelEvent>,
) -> Element {
    let title = item.title.clone();
    let tooltip = match item.status {
        TrayStatus::NeedsAttention => format!("{title} (needs attention)"),
        TrayStatus::Passive | TrayStatus::Active => title.clone(),
    };
    rsx! {
        // `data-wheel="capture"`: shell-host's scroll engine hands the raw wheel event only to
        // an element carrying it (design/11 §11.3.7), so a scroll over a tray icon still reaches
        // the item (TrayCommand::Scroll).
        span { "data-wheel": "capture", onwheel: move |event| onwheel.call(event),
            IconButton {
                common: Common { id: Some(popup_element_id(index)), ..Common::default() },
                variant: IconButtonVariant::Status,
                icon,
                label: title,
                tooltip: Some(tooltip),
                onclick,
            }
        }
    }
}

/// Open `item`'s dbusmenu as a `Menu{Dropdown}` popup anchored to the `popup_id` element.
fn open_menu(
    surface: &SurfaceHandle,
    popup_id: &str,
    item: &TrayItemId,
    menu: &[TrayMenuEntry],
    control: &Control<TrayCommand>,
) {
    // Fixed at the estimate: a document of one overlay measures at nothing, and shell-host's
    // content fit would shrink the popup to 1 x 1 (the bar's menus, `io.rs`).
    let entries = to_ds_entries(menu);
    let height = (menu_size(&entries).height + POPUP_SLACK).min(POPUP_MAX);
    let config = PopupConfig {
        size: PopupSize::Fixed,
        ..menu_popup(popup_id, MenuSide::Status, POPUP_MAX)
    };
    let placement = PopupPlacement {
        // Ignored for `PopupAnchor::Element` (shell-host's own doc on `PopupPlacement`).
        anchor_rect: Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        size: Size {
            width: POPUP_WIDTH,
            height,
        },
    };
    let root = HostRoot::with_props(
        tray_menu_popup,
        TrayMenuProps {
            entries,
            item: item.clone(),
            control: control.clone(),
        },
    );
    surface.open_popup(config, placement, root);
}

/// Props for [`tray_menu_popup`], the dbusmenu popup's own root — a plain `fn(P) -> Element`
/// (not `#[component]`), the shape `Root::with_props` needs (`shell-host/examples/demo_dock.rs`
/// is the precedent: `Root::with_props(dock, mode)`). Plain, not `#[component]`, because
/// `Control` has no `PartialEq` (module doc) and `#[component]` would demand one.
#[derive(Clone)]
struct TrayMenuProps {
    entries: Vec<MenuEntry<i32>>,
    item: TrayItemId,
    control: Control<TrayCommand>,
}

/// The dbusmenu popup's root: [`TrayMenuDocument`], a pick sent to the item's dbusmenu, a
/// close closing the popup. A popup inherits nothing from the bar, so it provides quire's rect
/// measurer itself.
fn tray_menu_popup(props: TrayMenuProps) -> Element {
    ds_native::measure::provide();
    let surface = use_surface();
    let control = props.control.clone();
    let item = props.item.clone();
    let onpick = move |entry: i32| {
        let _ = control.send(TrayCommand::MenuPick {
            item: item.clone(),
            entry,
        });
    };
    let onclose = move |()| surface.close();
    rsx! {
        TrayMenuDocument { entries: props.entries.clone(), onpick, onclose }
    }
}

/// `menu`'s shown items at the top level (a row's children are dropped, module doc), hidden
/// dbusmenu rows (`visibility: Hidden`) dropped.
fn to_ds_entries(menu: &[TrayMenuEntry]) -> Vec<MenuEntry<i32>> {
    menu.iter()
        .filter(|entry| match &entry.kind {
            TrayMenuKind::Separator => true,
            TrayMenuKind::Item { visibility, .. } => *visibility == Visibility::Shown,
        })
        .map(to_ds_entry)
        .collect()
}

fn to_ds_entry(entry: &TrayMenuEntry) -> MenuEntry<i32> {
    match &entry.kind {
        TrayMenuKind::Separator => MenuEntry::Separator,
        TrayMenuKind::Item {
            label,
            check,
            enabled,
            ..
        } => MenuEntry::Item {
            value: entry.id,
            title: label.clone(),
            detail: None,
            tile: None,
            trail: MenuTrail::None,
            check: to_ds_check(*check),
            availability: match enabled {
                TrayAvailability::Enabled => ds::Availability::Enabled,
                TrayAvailability::Disabled => ds::Availability::Disabled,
            },
        },
    }
}

fn to_ds_check(check: TrayCheck) -> Option<DsCheck> {
    match check {
        TrayCheck::None => None,
        TrayCheck::Unchecked => Some(DsCheck::Off),
        TrayCheck::Checked => Some(DsCheck::On),
    }
}

#[cfg(test)]
mod tests {
    use super::TrayButtonView;
    use dioxus::core::VirtualDom;
    use dioxus::prelude::*;
    use ds::lint::{LintConfig, Profile, Rule, markup};
    use sill_model::tray::{TrayIcon, TrayItem, TrayItemId, TrayStatus};

    fn fake_item(name: &str) -> TrayItem {
        TrayItem {
            id: TrayItemId(format!(":1.{name}/StatusNotifierItem")),
            title: name.to_owned(),
            icon: TrayIcon::None,
            status: TrayStatus::Active,
            menu: Vec::new(),
        }
    }

    /// Two fake tray entries, the way `Tray`'s own `for` loop would place them.
    fn two_fake_entries((a, b): (TrayItem, TrayItem)) -> Element {
        let icon = ds::IconSource::Glyph(ds::Icon::Window);
        rsx! {
            TrayButtonView { index: 0usize, item: a, icon: icon.clone(), onclick: move |_| {}, onwheel: move |_| {} }
            TrayButtonView { index: 1usize, item: b, icon, onclick: move |_| {}, onwheel: move |_| {} }
        }
    }

    fn render_ssr() -> String {
        let mut dom =
            VirtualDom::new_with_props(two_fake_entries, (fake_item("a"), fake_item("b")));
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    fn full_css() -> &'static str {
        ds::stylesheet()
    }

    #[test]
    fn two_fake_entries_render_clean_ds_markup() {
        let html = render_ssr();
        let offences = markup(
            &html,
            full_css(),
            &LintConfig {
                profile: Profile::Strict,
                ..LintConfig::new(&ds::kits())
            },
        );
        assert!(offences.is_empty(), "{offences:#?}");
    }

    #[test]
    fn no_raw_button_or_img_outside_ds_components() {
        let html = render_ssr();
        let offences = markup(&html, full_css(), &LintConfig::new(&ds::kits()));
        assert!(
            offences.iter().all(|o| o.rule != Rule::RawMarkup),
            "{offences:#?}"
        );
        assert!(!html.contains("<img"), "no raw <img>: {html}");
        assert!(html.contains("id=\"tray-0\""), "{html}");
        assert!(html.contains("id=\"tray-1\""), "{html}");
    }
}
