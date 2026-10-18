//! The panel's root.
//!
//! While the link holds an opening on this output the surface maps itself
//! (`SurfaceOp::SetVisibility`, [`super::io::use_stage`]) and a `Ds { material: Sheet, extent:
//! Viewport }` holds one `Sheet` per opening, keyed by its serial: the title, the line that says
//! what the panel is for, quire's `List` of `Row`s (the app's icon, its name, "(not responding)"
//! after a hung one's) and the Force Quit button. The button asks quire's `Alert` ("Do you want
//! to force quit ...?") whose Force Quit is destructive and whose Cancel is the default. Nothing
//! is styled under `.ds-sheet*`, `.ds-list*`, `.ds-row*`, `.ds-alert*` or `.ds-button*`.
//!
//! What the selection and the question do is [`super::step::advance`]; this file draws and
//! sends.

use dioxus::prelude::*;
use ds::base::vocab::RowState;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Answers;
use ds::components::lists::list::model::ListStyle;
use ds::components::lists::row::size::RowSize;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_launcher::AppId;
use sill_model::desktop::{ForceQuitRow, Standing};
use sill_ui_kit::app_icons::{LogicalPx, TileIcons, use_tile_icons};
use sill_ui_kit::links::{ForceQuitLink, ForceQuitOpening};
use sill_ui_kit::root::{ClickAway, SurfaceRoot};
use sill_ui_kit::sheet_stage::{SheetStage, settle};

use super::FORCE_QUIT_CSS;
use super::io::{use_rows, use_stage};
use super::layer::PANEL_ID;
use super::model::{Chosen, Confirming};
use super::step::{In, Out, advance};

/// The icon side of a row, in points.
const ROW_ICON_PX: u16 = 24;

/// The buttons' element ids (the tests', and a person's stylesheet's, handles).
const FORCE_QUIT_ID: &str = "force-quit-button";
const CONFIRM_ID: &str = "force-quit-confirm";
const CANCEL_ID: &str = "force-quit-cancel";

pub(super) fn force_quit(output: OutputInfo) -> Element {
    let stage = use_stage(&output);
    let drawn = stage().drawn().map(|(opening, shown)| (*opening, shown));
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: FORCE_QUIT_CSS }
            if let Some((opening, shown)) = drawn {
                PanelCard { key: "{opening.serial.0}", output, shown, stage }
            }
        }
    }
}

/// One opening's sheet and what the person has selected on it.
#[component]
fn PanelCard(
    output: OutputInfo,
    shown: Shown,
    stage: Signal<SheetStage<ForceQuitOpening>>,
) -> Element {
    let link = use_context::<ForceQuitLink>();
    let rows = use_rows();
    let icons = use_tile_icons(&output, LogicalPx(ROW_ICON_PX));
    let mut selection = use_signal(Chosen::default);
    let mut cursor = use_signal(|| None::<String>);

    // Every input goes through the pure step; what it asks for is carried out here.
    let send = use_callback({
        move |input: In| {
            let (next, out) = advance(selection.peek().clone(), input, &rows.peek());
            if *selection.peek() != next {
                selection.set(next);
            }
            match out {
                Out::Nothing => {}
                Out::Quit(app) => link.ask(app),
                Out::Close => link.close(),
            }
        }
    });
    // The list changed under the panel (or the panel opened on it): settle the selection.
    use_effect(move || {
        let _listed = rows.read();
        send.call(In::Listed);
    });

    let chosen = selection.read().clone();
    let items: Vec<ListItem<String>> = rows
        .read()
        .iter()
        .map(|row| {
            let key = row.app.clone();
            let title = title_of(row, &icons);
            let marked = chosen.app.as_deref() == Some(key.as_str());
            let content = rsx! {
                Row {
                    leading: RowLeading::Source(icons.source(&AppId(key.clone()))),
                    title: title.clone(),
                    state: RowState {
                        selection: if marked { Selection::Selected } else { Selection::Unselected },
                        ..RowState::default()
                    },
                    size: RowSize::Settings,
                    onclick: {
                        let key = key.clone();
                        move |_| {
                            cursor.set(Some(key.clone()));
                            send.call(In::Select(key.clone()));
                        }
                    },
                }
            };
            ListItem::row(key, title, content)
        })
        .collect();
    let empty = items.is_empty();
    let ready = if chosen.app.is_some() && chosen.confirming == Confirming::No {
        Availability::Enabled
    } else {
        Availability::Disabled
    };
    let asking = match &chosen.confirming {
        Confirming::App(app) => Some(name_of(app, &icons)),
        Confirming::No => None,
    };
    rsx! {
        // A sheet dims nothing and catches no click (macOS; design/30): a press away from the
        // panel is this surface's own, so it closes the panel as the scrim did.
        ClickAway { onpress: move |()| send.call(In::Escape) }
        Sheet {
            label: "Force Quit Applications".to_owned(),
            onclose: move |()| send.call(In::Escape),
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
            attach: Attach::Centre,
            width: SheetWidth::Regular,
            common: Common {
                id: Some(PANEL_ID.to_owned()),
                ..Common::default()
            },
            div {
                class: "sill-fq",
                tabindex: "0",
                onmounted: move |event: MountedEvent| focus_soon(event.data()),
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Escape {
                        send.call(In::Escape);
                    }
                },
                div { class: "sill-fq-title", "Force Quit Applications" }
                div { class: "sill-fq-byline",
                    "If an app doesn't respond for a while, select its name and click Force Quit."
                }
                div { class: "sill-fq-list",
                    if empty {
                        div { class: "sill-fq-empty", "No apps are open." }
                    } else {
                        List::<String> {
                            label: "Applications",
                            items,
                            style: ListStyle::Grouped,
                            cursor: cursor(),
                            onselect: move |key: String| {
                                cursor.set(Some(key.clone()));
                                send.call(In::Select(key));
                            },
                        }
                    }
                }
                div { class: "sill-fq-footer",
                    Button {
                        label: "Force Quit",
                        answers: Answers::Return,
                        availability: ready,
                        onclick: move |_| send.call(In::Ask),
                        common: Common { id: Some(FORCE_QUIT_ID.to_owned()), ..Common::default() },
                    }
                }
            }
        }
        if let Some(name) = asking {
            Question { name, send }
        }
    }
}

/// "Do you want to force quit ...?" over the panel: Cancel is the default (Return), Force Quit
/// is destructive.
#[component]
fn Question(name: String, send: Callback<In>) -> Element {
    let buttons = vec![
        AlertButton::new(
            "Cancel",
            AlertRole::Cancel,
            EventHandler::new(move |()| send.call(In::Decline)),
        )
        .with_common(Common {
            id: Some(CANCEL_ID.to_owned()),
            ..Common::default()
        }),
        AlertButton::new(
            "Force Quit",
            AlertRole::Destructive,
            EventHandler::new(move |()| send.call(In::Confirm)),
        )
        .with_common(Common {
            id: Some(CONFIRM_ID.to_owned()),
            ..Common::default()
        }),
    ];
    rsx! {
        Alert {
            title: format!("Do you want to force quit \u{201c}{name}\u{201d}?"),
            message: Some(TextLine::from("You will lose any unsaved changes.")),
            buttons,
        }
    }
}

/// An app's name as the person knows it: its desktop entry's, else its key.
fn name_of(app: &str, icons: &TileIcons) -> String {
    icons
        .index
        .get(&AppId(app.to_owned()))
        .map_or_else(|| app.to_owned(), |entry| entry.name.clone())
}

/// A row's words: the name, then "(not responding)" for an app that does not answer.
fn title_of(row: &ForceQuitRow, icons: &TileIcons) -> String {
    let name = name_of(&row.app, icons);
    match row.standing {
        Standing::Responding => name,
        Standing::NotResponding => format!("{name} (not responding)"),
    }
}
