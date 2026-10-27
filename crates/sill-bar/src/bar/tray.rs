//! The tray: one `Button{Status}` per status notifier item, its dbusmenu one more menu of the
//! bar's session.
//!
//! Each item draws its own icon: a pixmap or a theme icon through `sill_services::icon_art`,
//! symbolic (drawn in the frame's ink, like a glyph) or an image by design/08-ICONS.md §1.5, as
//! `tray_input::icon_source` decides with quire's `ds::style::icon::classify::classify_with`. The button carries the
//! `tray-<n>` id and hands its pointer to the session (`BarMenu::Tray`), so an item with a
//! dbusmenu opens it on the press, switches to it on hover from another menu and shows it open
//! as every bar item does; `dbusmenu` turns the rows into entries, submenus included, and the
//! bar's one popup draws them. An item without one takes the click as SNI `Activate` and the
//! secondary press as `ContextMenu`; the middle button is `SecondaryActivate` on either, each
//! with the press's screen point. The wheel over an item is SNI `Scroll`.
//!
//! `TrayButtonView` carries no hook: it is the part `tests` renders directly. `TrayButton`
//! wraps it with the hooks a live surface needs (the session's pointer, the tray `Control`),
//! which a plain unit test cannot construct (`Control` has no `PartialEq`, so it cannot be a
//! `#[component]` prop at all: `TrayButton` reads it itself instead of receiving it).

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::components::controls::button_model::ImagePosition;
use ds::host::measure::use_rect;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::icon::classify::ChromaLimit;
use shell_host::OutputInfo;
use sill_model::tray::{TrayCommand, TrayIcon, TrayItem, TrayStatus};
use sill_services::TrayService;
use sill_services::tray::icon_art;
use sill_shell_kit::bar::pointer::BarPointer;
use sill_shell_kit::prelude::*;

use super::dbusmenu::has_rows;
use super::menus::{BarMenu, TraySlot};
use super::title::{shown_of, use_title_pointer};
use super::tray_input::{
    ItemMenu, ScreenOrigin, TrayAction, icon_source, press_action, scroll_delta,
};
use sill_ui_kit::hooks::{use_control, use_service, use_settings};

/// The element id of the button at `index` (`tray-<n>`, as the module doc says).
fn popup_element_id(index: usize) -> String {
    format!("tray-{index}")
}

/// The tray: one `Button{Status}` per status notifier item. The items are global, not per-output, and each output's bar is its own
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

/// One status item, wired to the live bar: its pointer is the session's when it has a dbusmenu,
/// a press does what `press_action` says, and the wheel sends `Scroll`.
#[component]
fn TrayButton(index: usize, item: TrayItem, origin: ScreenOrigin) -> Element {
    let control = use_control::<TrayService>();
    let size_px = use_settings()().bar.status_icon_box_px.0;
    let limit = ChromaLimit::PROPOSED;
    let icon = use_icon_source(&item.icon, size_px, limit);
    let probe = use_rect();
    let menu = BarMenu::Tray(TraySlot(u16::try_from(index).unwrap_or(u16::MAX)));
    let session = use_title_pointer(menu, probe);
    let shown = shown_of(menu);
    let item_id = item.id.clone();
    let (has_menu, pointer, shown) = if has_rows(&item.menu) {
        (ItemMenu::Present, session, shown)
    } else {
        (ItemMenu::Absent, BarPointer::default(), Shown::Hidden)
    };
    let onclick = {
        let control = control.clone();
        let item_id = item_id.clone();
        move |press: Press| match press_action(press, &item_id, has_menu, origin) {
            TrayAction::Send(command) => {
                // The service may have stopped; then nobody is left to tell.
                let _gone_is_dropped = control.send(command);
            }
            TrayAction::OpenMenu => {}
        }
    };
    let onwheel = move |event: WheelEvent| {
        if let Some(delta) = scroll_delta(event.delta()) {
            let _gone_is_dropped = control.send(TrayCommand::Scroll {
                item: item_id.clone(),
                delta,
            });
        }
    };
    let common = Common {
        mounted: Some(EventHandler::new(move |event| probe.on_mounted(event))),
        ..Common::default()
    };
    rsx! {
        TrayButtonView { index, item, icon, shown, pointer, common, onclick, onwheel }
    }
}

/// One status item's markup: no hook, so `tests` can render it directly. The unclassed `span`
/// only listens for the wheel, which `Button` does not report.
#[component]
fn TrayButtonView(
    index: usize,
    item: TrayItem,
    icon: IconSource,
    shown: Shown,
    pointer: BarPointer,
    common: Common,
    onclick: EventHandler<Press>,
    onwheel: EventHandler<WheelEvent>,
) -> Element {
    let title = item.title.clone();
    // The tip is the name alone (quire's tip house style); the accessible name keeps the status.
    let spoken = match item.status {
        TrayStatus::NeedsAttention => Some(format!("{title}, needs attention")),
        TrayStatus::Passive | TrayStatus::Active => None,
    };
    let common = Common {
        aria_label: spoken.or_else(|| common.aria_label.clone()),
        ..common
    };
    rsx! {
        // `data-wheel="capture"`: shell-host's scroll engine hands the raw wheel event only to
        // an element carrying it (design/11 §11.3.7), so a scroll over a tray icon still reaches
        // the item (TrayCommand::Scroll).
        span { "data-wheel": "capture", onwheel: move |event| onwheel.call(event),
            MenuBarItem {
                common: Common { id: Some(popup_element_id(index)), ..common },
                image: ImagePosition::Only,
                icon,
                title: Some(title.clone()),
                label: title,
                shown,
                pointer,
                onclick,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TrayButtonView;
    use dioxus::core::VirtualDom;
    use dioxus::prelude::*;
    use ds::prelude::*;
    use ds::root::common::Common;
    use ds_lint::{LintConfig, Profile, Rule, markup};
    use sill_model::tray::{TrayIcon, TrayItem, TrayItemId, TrayStatus};
    use sill_shell_kit::bar::pointer::BarPointer;

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
        let icon = IconSource::Glyph(Icon::Window);
        rsx! {
            TrayButtonView { index: 0usize, item: a, icon: icon.clone(), shown: Shown::Hidden, pointer: BarPointer::default(), common: Common::default(), onclick: move |_| {}, onwheel: move |_| {} }
            TrayButtonView { index: 1usize, item: b, icon, shown: Shown::Visible, pointer: BarPointer::default(), common: Common::default(), onclick: move |_| {}, onwheel: move |_| {} }
        }
    }

    fn render_ssr() -> String {
        let mut dom =
            VirtualDom::new_with_props(two_fake_entries, (fake_item("a"), fake_item("b")));
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    fn full_css() -> &'static str {
        sill_shell_kit::stylesheet()
    }

    /// The tip is the name alone; only the accessible name says the item needs attention.
    #[test]
    fn a_tip_is_the_name_and_the_accessible_name_keeps_the_status() {
        const CASES: [(TrayStatus, &str); 2] = [
            (TrayStatus::Active, "aria-label=\"Sync\""),
            (
                TrayStatus::NeedsAttention,
                "aria-label=\"Sync, needs attention\"",
            ),
        ];
        for (status, aria) in CASES {
            let item = TrayItem {
                status,
                ..fake_item("Sync")
            };
            let mut dom = VirtualDom::new_with_props(two_fake_entries, (item.clone(), item));
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains(aria), "{status:?}: {html}");
            assert!(html.contains("title=\"Sync\""), "{status:?}: {html}");
        }
    }

    #[test]
    fn two_fake_entries_render_clean_ds_markup() {
        let html = render_ssr();
        let offences = markup(
            &html,
            full_css(),
            &LintConfig {
                profile: Profile::Strict,
                ..LintConfig::new(&sill_shell_kit::kits())
            },
        );
        assert!(offences.is_empty(), "{offences:#?}");
    }

    #[test]
    fn no_raw_button_or_img_outside_ds_components() {
        let html = render_ssr();
        let offences = markup(&html, full_css(), &LintConfig::new(&sill_shell_kit::kits()));
        assert!(
            offences.iter().all(|o| o.rule != Rule::RawMarkup),
            "{offences:#?}"
        );
        assert!(!html.contains("<img"), "no raw <img>: {html}");
        assert!(html.contains("id=\"tray-0\""), "{html}");
        assert!(html.contains("id=\"tray-1\""), "{html}");
    }
}
