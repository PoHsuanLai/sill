//! GroupHeader: the head of one app's notifications in the notification center (design/20 section
//! 1.6, design/13 section 13.3.6 "Grouping by app"). The app's icon at 16,
//! its name and how many it holds in `SectionHeader`'s group type (the data face in capitals at
//! the caption size, `--ink-faint`), then the trailing pair: the toggle that folds the group to
//! its newest ("N more" while folded, "Show less" while open) and Clear. Both are Quiet buttons
//! that keep their press, so a header inside a pressable group row never also presses the row.

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::vocab::Shown;
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Bezel;
use ds::components::controls::press::Propagation;
use ds::root::common::Common;
use ds::style::icon::render::IconSize;

/// The toggle's words: "Show less" while the group is open; while folded to its newest, how
/// many are hidden ("2 more"), or nothing when there is nothing to show.
fn toggle_label(expanded: Shown, count: u32) -> Option<String> {
    match (expanded, count) {
        (Shown::Visible, _) => Some("Show less".to_owned()),
        (Shown::Hidden, 0 | 1) => None,
        (Shown::Hidden, n) => Some(format!("{} more", n - 1)),
    }
}

/// One app's group header. `count` is how many notifications the group holds; `expanded` is
/// whether the group shows them all (`aria-expanded` on the toggle). `on_toggle` folds or opens
/// it; `on_clear` clears the group.
#[component]
pub fn GroupHeader(
    icon: IconSource,
    #[props(into)] name: TextLine,
    count: u32,
    #[props(default)] expanded: Shown,
    on_toggle: EventHandler<Press>,
    on_clear: EventHandler<Press>,
    #[props(default)] common: Common,
) -> Element {
    let clear = format!("Clear {}", name.plain_text());
    let class = common.class("ds-group-header");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-expanded": expanded.slug(),
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            span { class: "ds-group-header-icon",
                IconView { source: icon, size: IconSize::Base }
            }
            span { class: "ds-group-header-name", {text(&name)} }
            if count > 0 {
                span { class: "ds-group-header-count", "{count}" }
            }
            span { class: "ds-group-header-actions",
                if let Some(label) = toggle_label(expanded, count) {
                    Button {
                        bezel: Bezel::Inline,
                        label,
                        shown: expanded,
                        propagation: Propagation::Stop,
                        onclick: move |press| on_toggle.call(press),
                    }
                }
                Button {
                    common: Common { aria_label: Some(clear), ..Common::default() },
                    bezel: Bezel::Inline,
                    label: "Clear",
                    propagation: Propagation::Stop,
                    onclick: move |press| on_clear.call(press),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::toggle_label;
    use ds::base::vocab::Shown;

    #[test]
    fn the_toggle_names_what_it_will_do() {
        let cases = [
            (Shown::Visible, 4, Some("Show less")),
            (Shown::Visible, 1, Some("Show less")),
            (Shown::Hidden, 4, Some("3 more")),
            (Shown::Hidden, 2, Some("1 more")),
            (Shown::Hidden, 1, None),
            (Shown::Hidden, 0, None),
        ];
        for (expanded, count, want) in cases {
            assert_eq!(
                toggle_label(expanded, count).as_deref(),
                want,
                "{expanded:?} {count}"
            );
        }
    }
}
