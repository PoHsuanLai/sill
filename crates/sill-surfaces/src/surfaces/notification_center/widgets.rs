//! The widgets slot below the history (macOS's Notification Center widgets; design/20 §1.14):
//! one [`Widget`] in the center host for each of `widgets.center`, in order, each at its one
//! size in the center ([`size_of`], Medium) on quire's card (a tile on the Popover), then "Edit
//! Widgets" at the bottom, as macOS's Notification Center has it: it closes the center and opens the Edit
//! Widgets sheet on this output (`surfaces::edit_widgets`). The panel injects sill's widgets'
//! stylesheets (`widgets_css`).

use dioxus::prelude::*;
use ds::{Button, ButtonVariant};
use shell_host::OutputId;

use super::drive::CenterCtx;
use crate::hooks::use_settings;
use crate::links::EditWidgetsLink;
use crate::widgets::{Widget, WidgetHost};

/// The widgets under the history, as `widgets.center` lists them, and the way to edit them.
#[component]
pub fn CenterWidgets(output: OutputId) -> Element {
    let widgets = use_settings()().widgets;
    let ctx = try_use_context::<CenterCtx>();
    let edit = try_use_context::<EditWidgetsLink>();
    rsx! {
        for (at, kind) in widgets.center.into_iter().enumerate() {
            Widget {
                key: "{at}-{kind:?}",
                kind,
                host: WidgetHost::Tile,
            }
        }
        if let Some(edit) = edit {
            div { class: "sill-nc-edit",
                Button {
                    variant: ButtonVariant::Quiet,
                    label: "Edit Widgets",
                    onclick: move |_| {
                        if let Some(ctx) = &ctx {
                            ctx.close();
                        }
                        edit.open(output);
                    },
                }
            }
        }
    }
}
