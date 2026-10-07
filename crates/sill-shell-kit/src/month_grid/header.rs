//! A MonthGrid's header: the month's title and, with `onstep`, the previous and next buttons
//! (design/04-COMPONENTS.md section 39). The regular grid uses `Button { Tool }` (28 x 26);
//! that alone is taller than the compact grid can spare in a small widget's 140 px,
//! so the compact grid draws plain 14 px glyph buttons (`ds-month-step`, an 11 px glyph).

use crate::month_grid::data::MonthStep;
use crate::month_grid::density::Drawn;
use dioxus::prelude::*;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button_model::{Bezel, ImagePosition};
use ds::prelude::*;
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconSize};
use ds::style::tokens::control_size::ControlSize;

/// The header row at `density`.
pub(crate) fn header(
    title: &TextLine,
    density: Drawn,
    onstep: Option<EventHandler<MonthStep>>,
) -> Element {
    rsx! {
        div { class: "ds-month-header",
            span { class: "ds-month-title", {text(title)} }
            if let Some(onstep) = onstep {
                {step_button(MonthStep::Previous, Icon::ChevronLeft, density, onstep)}
                {step_button(MonthStep::Next, Icon::ChevronRight, density, onstep)}
            }
        }
    }
}

/// One of the header's step buttons, sized for `density`.
fn step_button(
    step: MonthStep,
    icon: Icon,
    density: Drawn,
    onstep: EventHandler<MonthStep>,
) -> Element {
    match density {
        Drawn::Regular => rsx! {
            Button {
                bezel: Bezel::Toolbar, size: ControlSize::Large, image: ImagePosition::Only,
                icon,
                label: step.label(),
                title: step.label().to_owned(),
                onclick: move |_| onstep.call(step),
            }
        },
        Drawn::Compact => rsx! {
            button {
                r#type: "button",
                class: "ds-month-step",
                "aria-label": step.label(),
                title: step.label(),
                onclick: move |_| onstep.call(step),
                Glyph { icon, size: IconSize::Micro }
            }
        },
    }
}
