//! The tray: one `IconButton{Tool}` per status notifier item, its dbusmenu as a quire `Menu`
//! (M1 svc-tray owns this file).
//!
//! **A per-item icon is a stand-in (`Icon::Window`), not the item's own icon.** `ds::Icon` is a
//! closed enum and `IconButton`'s `Glyph` only ever draws one of its variants; there is nowhere
//! to put a pixmap's PNG or a theme's symbolic SVG. design/08-ICONS.md section 1.5 already names
//! the mechanics for this exact case ("Blitz: the recoloured tray icon is a `mask-image` data
//! URI with `background: currentColor`"), but no `ds` component renders one yet
//! (`sill_services::tray::icon_url` already resolves the real URL, ready for when one exists).
//! `ds::MenuEntry` also has no `children` field (no nested submenus) and no "disabled" field, so
//! a dbusmenu item's submenu is dropped and a disabled item renders exactly like an enabled one.
//! FINDINGS.md "M1 svc-tray" has the detail (quire gap Q6).
//!
//! **The popup anchor is a plain `span[id]`, not a ds component.** `IconButton` exposes no `id`
//! prop, so there is nowhere on its own `<button>` to put the `tray-<n>` id the freeze asks
//! popups to anchor by; a bare `span` with no class carries nothing the markup lint checks, so
//! this is the narrowest wrapper that satisfies both. `bar/popup.rs`'s `menu_popup` and
//! `menu_offset` (done, pure) place the popup itself; the ds `Menu` inside it is given its own
//! synthetic anchor rect the same width as the popup's initial size guess, so the two line up
//! (see `POPUP_WIDTH`). This whole composition is provisional: "see how the bar's other menus do
//! it once bar-ui lands" (the M1 svc-tray brief).
//!
//! `TrayButtonView` carries no hook: it is the part `tests` renders directly. `TrayButton`
//! wraps it with the two hooks a live surface needs (`use_surface`, the tray `Control`), neither
//! of which a plain unit test can construct (`SurfaceHandle`'s constructor is
//! `pub(crate)` to shell-host, and `Control` has no `PartialEq`, so it cannot be a `#[component]`
//! prop at all — `TrayButton` reads it itself instead of receiving it).

use dioxus::prelude::*;
use ds::icon::Icon;
use ds::{
    Anchor, Check as DsCheck, Ds, IconButton, IconButtonVariant, Material, Menu, MenuEntry,
    MenuKind, Point, Px, Rect as DsRect, Size as DsSize, Trail,
};
use shell_host::{OutputInfo, PopupPlacement, Rect, Root, Size, SurfaceHandle, use_surface};
use sill_services::{
    Check as TrayCheck, Control, TrayCommand, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind,
    TrayStatus, Visibility,
};

use super::popup::{MenuSide, menu_popup};
use crate::hooks::{use_appearance, use_services, use_tray};

/// The popup's guessed initial width (design/04-COMPONENTS.md section 20, Dropdown: "min 186,
/// auto"), before its content is measured — and, doubling as the inner `Menu`'s synthetic
/// anchor rect width, so the menu's right edge lands at the popup's own right edge (module doc).
const POPUP_WIDTH: i32 = 220;
/// The popup's guessed initial height, before its content is measured.
const POPUP_HEIGHT: i32 = 240;
/// `menu_popup`'s `max_px`: how far the popup may grow past `POPUP_WIDTH`/`POPUP_HEIGHT` once
/// its dbusmenu content is known.
const POPUP_MAX: i32 = 320;

/// The element id a popup anchors to for the button at `index` (module doc: "anchor by element
/// id `tray-<n>` per the freeze").
fn popup_element_id(index: usize) -> String {
    format!("tray-{index}")
}

/// The tray: one `IconButton{Tool}` per status notifier item, its dbusmenu as a quire
/// `Menu{Dropdown}`. `output` is unused: the tray's items are global, not per-output, and each
/// output's bar is its own document, so `tray-<n>` needs no output qualifier.
#[component]
#[allow(unused_variables)]
pub fn Tray(output: OutputInfo) -> Element {
    let tray = use_tray();
    let items = tray().items;
    rsx! {
        for (index , item) in items.into_iter().enumerate() {
            TrayButton { key: "{item.id.0}", index, item }
        }
    }
}

/// One status item, wired to the live surface: primary click opens its dbusmenu when it has
/// one, otherwise sends `Activate` (SNI's default action); a picked entry sends `MenuPick`.
/// `(x, y)` on `Activate`/`MenuOpening` are the SNI spec's own words for the screen coordinates:
/// "a hint... where to show eventual windows (if any)" — not load-bearing, so `0, 0` stands in
/// until the bar has a reason to plumb the real pointer position through here.
#[component]
fn TrayButton(index: usize, item: TrayItem) -> Element {
    let surface = use_surface();
    let control = use_services().tray_control;
    let popup_id = popup_element_id(index);
    let menu = item.menu.clone();
    let item_id = item.id.clone();
    let onclick = move |()| {
        if menu.is_empty() {
            let _ = control.send(TrayCommand::Activate {
                item: item_id.clone(),
                x: 0,
                y: 0,
            });
            return;
        }
        let _ = control.send(TrayCommand::MenuOpening {
            item: item_id.clone(),
        });
        open_menu(&surface, &popup_id, &item_id, &menu, &control);
    };
    rsx! {
        TrayButtonView { index, item, onclick }
    }
}

/// One status item's markup: no hook, so `tests` can render it directly.
#[component]
fn TrayButtonView(index: usize, item: TrayItem, onclick: EventHandler<()>) -> Element {
    let title = item.title.clone();
    let tooltip = match item.status {
        TrayStatus::NeedsAttention => format!("{title} (needs attention)"),
        TrayStatus::Passive | TrayStatus::Active => title.clone(),
    };
    rsx! {
        span { id: "{popup_element_id(index)}",
            IconButton {
                variant: IconButtonVariant::Tool,
                icon: Icon::Window,
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
    let config = menu_popup(popup_id, MenuSide::Status, POPUP_MAX);
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
            height: POPUP_HEIGHT,
        },
    };
    let root = Root::with_props(
        tray_menu_popup,
        TrayMenuProps {
            entries: to_ds_entries(menu),
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

/// The dbusmenu popup's document: one `Ds` over one `Menu{Dropdown}`.
fn tray_menu_popup(props: TrayMenuProps) -> Element {
    let environment = use_appearance()();
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
        Ds {
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            material: Material::Popover,
            Menu {
                kind: MenuKind::Dropdown,
                anchor: Anchor::Rect(DsRect {
                    origin: Point::default(),
                    size: DsSize {
                        width: Px(POPUP_WIDTH as f32),
                        height: Px(0.0),
                    },
                }),
                entries: props.entries.clone(),
                onpick,
                onclose,
            }
        }
    }
}

/// `menu`'s shown items, flattened to one level (`ds::MenuEntry` has no `children` field, module
/// doc), hidden dbusmenu rows (`visibility: Hidden`) dropped.
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
        TrayMenuKind::Item { label, check, .. } => MenuEntry::Item {
            value: entry.id,
            title: label.clone(),
            detail: None,
            tile: None,
            trail: Trail::None,
            check: to_ds_check(*check),
            availability: ds::Availability::Enabled,
        },
    }
}

fn to_ds_check(check: TrayCheck) -> Option<DsCheck> {
    match check {
        TrayCheck::None => None,
        TrayCheck::Unchecked => Some(DsCheck::Unchecked),
        TrayCheck::Checked => Some(DsCheck::Checked),
    }
}

#[cfg(test)]
mod tests {
    use super::TrayButtonView;
    use dioxus::core::VirtualDom;
    use dioxus::prelude::*;
    use ds::lint::{LintConfig, Profile, Rule, markup};
    use sill_services::{TrayIcon, TrayItem, TrayItemId, TrayStatus};

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
        rsx! {
            TrayButtonView { index: 0usize, item: a, onclick: move |()| {} }
            TrayButtonView { index: 1usize, item: b, onclick: move |()| {} }
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
                ..LintConfig::default()
            },
        );
        assert!(offences.is_empty(), "{offences:#?}");
    }

    #[test]
    fn no_raw_button_or_img_outside_ds_components() {
        let html = render_ssr();
        let offences = markup(&html, full_css(), &LintConfig::default());
        assert!(
            offences.iter().all(|o| o.rule != Rule::RawMarkup),
            "{offences:#?}"
        );
        assert!(!html.contains("<img"), "no raw <img>: {html}");
        assert!(html.contains("id=\"tray-0\""), "{html}");
        assert!(html.contains("id=\"tray-1\""), "{html}");
    }
}
