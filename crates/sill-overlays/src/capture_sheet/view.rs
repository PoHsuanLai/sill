//! The picker's root.
//!
//! While the capture service holds a request and the `BannerLink` names this output, the surface
//! maps itself (`SurfaceOp::SetVisibility`) and a `Ds { material: Sheet, extent: Viewport }` holds
//! one `Sheet` per request, keyed by its id: the title, who asks, a segmented control for the
//! kinds the app accepts (only when both), quire's `List` of `Row`s for what can be shared, a
//! segmented control for the pointer (only when the app accepts more than one way), and the
//! buttons. Nothing is styled under `.ds-sheet*`, `.ds-list*`, `.ds-row*` or `.ds-button*`.
//!
//! The choices come from the pure `sill_model::capture_pick::plan`, recomputed as windows and
//! screens come and go; what the person has picked is a `Picks`, changed only by `toggle`.

use dioxus::prelude::*;
use ds::base::vocab::RowState;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Answers;
use ds::components::controls::segmented::Tracking;
use ds::components::lists::list::model::ListStyle;
use ds::components::lists::row::size::RowSize;
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::capture_pick::{
    Asking, Outcome, Picks, Plurality, Pointer, Source, Tab, can_allow, plan, toggle,
};
use sill_services::capture_pick::CaptureService;

use super::CAPTURE_SHEET_CSS;
use super::io::use_end;
use super::layer::PANEL_ID;
use super::rows::{byline_of, empty_words, key_of, pointer_words, rows_of, source_of, tab_words};
use sill_ui_kit::hooks::{use_banner_output, use_service_part};
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

pub(super) fn capture_sheet(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let drawn = stage()
        .drawn()
        .map(|(asking, shown)| (asking.clone(), shown));
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: CAPTURE_SHEET_CSS }
            if let Some((asking, shown)) = drawn {
                SheetCard { key: "{asking.id.0}", asking, shown, stage }
            }
        }
    }
}

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
const ALLOW_ID: &str = "capture-allow";
const CANCEL_ID: &str = "capture-cancel";

/// One request's sheet and what the person has picked on it.
#[component]
fn SheetCard(asking: Asking, shown: Shown, stage: Signal<SheetStage<Asking>>) -> Element {
    let candidates = use_service_part::<CaptureService, _>(|picker| picker.candidates.clone());
    let end = use_end(asking.id);
    let ask = asking.ask;
    let now = plan(&ask, &candidates());
    let mut picks = use_signal({
        let now = now.clone();
        move || Picks::start(&now)
    });
    let mut cursor = use_signal(|| None::<String>);
    let plurality = ask.plurality;

    let press = use_callback({
        let ask = ask.clone();
        move |source: Source| {
            let now = plan(&ask, &candidates.peek());
            let next = toggle(&now, ask.plurality, picks.peek().clone(), source);
            picks.set(next);
        }
    });
    let allow = use_callback({
        let ask = ask.clone();
        move |(): ()| {
            let now = plan(&ask, &candidates.peek());
            let current = picks.peek().clone();
            if can_allow(&now, &current) {
                end.call(Outcome::Allow(current));
            }
        }
    });

    let picked = picks();
    let tab = picked.tab;
    let rows = rows_of(&now, tab, &picked);
    let items: Vec<ListItem<String>> = rows
        .into_iter()
        .map(|row| {
            let source = row.source.clone();
            let content = rsx! {
                Row {
                    leading: RowLeading::Icon(row.icon),
                    title: row.title.clone(),
                    detail: (!row.detail.is_empty()).then(|| TextLine::from(row.detail.clone())),
                    accessory: Accessory::Check(row.check),
                    state: RowState { selection: row.selection, ..RowState::default() },
                    size: RowSize::Settings,
                    onclick: move |_| {
                        cursor.set(Some(key_of(&source)));
                        press.call(source.clone());
                    },
                }
            };
            ListItem::row(key_of(&row.source), row.title, content)
        })
        .collect();

    let ready = can_allow(&now, &picked);
    let permit = if ready {
        Availability::Enabled
    } else {
        Availability::Disabled
    };
    let title = format!("Choose what to share with {}", ask.app_label);
    let byline = byline_of(&now, plurality);
    let on_move = {
        let now = now.clone();
        move |key: String| {
            cursor.set(Some(key.clone()));
            // With one source allowed the arrows pick as they move, as a Finder list does.
            if plurality == Plurality::One
                && let Some(source) = source_of(&now, &key)
            {
                press.call(source);
            }
        }
    };
    let on_space = {
        let now = now.clone();
        move |event: KeyboardEvent| match event.key() {
            Key::Enter => {
                event.prevent_default();
                allow.call(());
            }
            Key::Character(text) if text == " " => {
                if let Some(source) = cursor.peek().as_ref().and_then(|key| source_of(&now, key)) {
                    event.prevent_default();
                    press.call(source);
                }
            }
            _ => {}
        }
    };
    let listing = now.on(tab).is_empty();
    rsx! {
        Sheet {
            label: title.clone(),
            onclose: move |()| end.call(Outcome::Cancel),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            attach: Attach::Centre,
            width: SheetWidth::Regular,
            common: Common {
                id: Some(PANEL_ID.to_owned()),
                ..Common::default()
            },
            div {
                class: "sill-capture",
                tabindex: "0",
                onmounted: move |event: MountedEvent| focus_soon(event.data()),
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Escape {
                        end.call(Outcome::Cancel);
                    }
                },
                div { class: "sill-capture-title", "{title}" }
                div { class: "sill-capture-byline", "{byline}" }
                if now.tabs.len() > 1 {
                    div { class: "sill-capture-tabs",
                        SegmentedControl::<Tab> {
                            label: "Share",
                            choices: now.tabs.iter().map(|tab| Choice::new(*tab, tab_words(*tab))).collect::<Vec<_>>(),
                            tracking: Tracking::SelectOne(tab),
                            onchange: move |next: Tab| picks.with_mut(|p| p.tab = next),
                        }
                    }
                }
                div { class: "sill-capture-list", onkeydown: on_space,
                    if listing {
                        div { class: "sill-capture-empty", {empty_words(tab)} }
                    } else {
                        List::<String> {
                            label: tab_words(tab),
                            items,
                            style: ListStyle::Grouped,
                            cursor: cursor(),
                            onselect: on_move,
                        }
                    }
                }
                if now.pointers.len() > 1 {
                    div { class: "sill-capture-pointer",
                        span { class: "sill-capture-pointer-label", "Pointer" }
                        SegmentedControl::<Pointer> {
                            label: "Pointer",
                            choices: now.pointers.iter().map(|p| Choice::new(*p, pointer_words(*p))).collect::<Vec<_>>(),
                            tracking: Tracking::SelectOne(picked.pointer),
                            onchange: move |next: Pointer| picks.with_mut(|p| p.pointer = next),
                        }
                    }
                }
                div { class: "sill-capture-footer",
                    Button {
                        label: "Cancel",
                        answers: Answers::Escape,
                        onclick: move |_| end.call(Outcome::Cancel),
                        common: Common { id: Some(CANCEL_ID.to_owned()), ..Common::default() },
                    }
                    Button {
                        label: "Share",
                        answers: Answers::Return,
                        availability: permit,
                        onclick: move |_| allow.call(()),
                        common: Common { id: Some(ALLOW_ID.to_owned()), ..Common::default() },
                    }
                }
            }
        }
    }
}

/// The stage on `output`, following the request and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<Asking>> {
    let asking = use_service_part::<CaptureService, _>(|picker| picker.asking.clone());
    let shows = use_banner_output();
    use_sheet_stage(move || {
        let current = asking();
        shows
            .read()
            .shows_on(output.id)
            .then_some(current)
            .flatten()
    })
}
