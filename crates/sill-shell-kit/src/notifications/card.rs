//! NotificationCard: one notification, as a banner or a row of the notification center (design/20
//! section 1.6, design/13 section 13.3.6).
//!
//!
//! The card is a plate in its material (Toast by default) inside a scope of that material whose
//! own box paints nothing (`Surface { chrome: Some(RootChrome::Transparent) }`), so the same card
//! paints the Toast material in a Toast root or in a Popover-rooted center. The plate holds the
//! app's icon at 32, the summary (13/700, one line) with the group's count chip and the age (the
//! data face at the caption size) on its first line, the body under it, and the actions. A
//! press on the plate opens the notification (`on_open`); the close button and every action and
//! link keep their press, so none of them also opens it.
//!
//! Under the pointer (`data-hover="on"`, told to `on_hover` so the caller's hold timer can
//! pause) the body opens from two lines to six over `--t-move --e-out`, the actions row opens
//! under it, and the 18 px close button shows at the top left corner, as macOS draws it. Keyboard
//! focus on the close button shows it too.
//!
//! A group (`count`) shows its count in a chip and draws `layers` offset plates behind the card,
//! each `--notifications-group-offset` lower and a little narrower.
//! `swipe: Swipe::Dismiss(on_dismiss)` lets a drag or a horizontal scroll to the right dismiss
//! the card (`notification_swipe`); the click that ends a drag never opens it.

use crate::notifications::body::NotificationBody;
use crate::notifications::parts::{AppMark, CardAction, GroupCount, Hover};
use crate::notifications::swipe::{NotificationSwipe, use_card_swipe};
use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::word::Word;
use ds::components::content::icon_view::IconView;
use ds::components::content::rich_text::Rich;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button::Button;
use ds::components::controls::press::{ActivationKeys, PressListeners, Propagation};
use ds::motion::swipe::SwipeMetrics;
use ds::root::chrome::RootChrome;
use ds::root::common::Common;
use ds::root::surface::Surface;
use ds::style::appearance::material::Material;
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconPx, IconSize};
use ds::style::tokens::control_size::ControlSize;

/// One notification. `app`, `age` (the caller's words: "now", "2m") and `summary` are its first
/// line; `body` (runs and links, clamped) is optional, as are the group's `count` and the
/// `actions`. `on_open` hears a press on the card, Enter or Space; `on_close` a press on the
/// close button; `on_link` a press on a link in the body. `icon_size` is the icon's side
/// (`notifications.icon_px`, 32). `common.id` names the plate, the painted box, for its blur region
/// (`Element("toast-<id>")`); the consumer's classes, data and label go on the card's root. `swipe` turns swipe to dismiss on, with `swipe_metrics` from the
/// `notifications.swipe_*` keys.
#[component]
pub fn NotificationCard(
    app: AppMark,
    #[props(into)] age: TextLine,
    #[props(into)] summary: TextLine,
    #[props(default)] body: Option<Rich>,
    #[props(default)] count: Option<GroupCount>,
    #[props(default)] actions: Vec<CardAction>,
    on_close: EventHandler<Press>,
    on_open: EventHandler<Press>,
    #[props(default)] on_link: Option<EventHandler<String>>,
    #[props(default)] on_hover: Option<EventHandler<Hover>>,
    #[props(default = Material::Toast)] material: Material,
    #[props(default = IconPx(32))] icon_size: IconPx,
    #[props(default)] common: Common,
    #[props(default)] swipe: NotificationSwipe,
    #[props(default)] swipe_metrics: SwipeMetrics,
) -> Element {
    let swiper = use_card_swipe(&swipe, swipe_metrics);
    let mut hover = use_signal(Hover::default);
    let mut point = move |next: Hover| {
        if *hover.peek() != next {
            hover.set(next);
            if let Some(on_hover) = on_hover {
                on_hover.call(next);
            }
        }
    };
    let layers = count.map_or(0, |group| group.layers.drawn());
    let spoken = format!("{}: {}", app.name.plain_text(), summary.plain_text());
    let label = common.aria_label.clone().unwrap_or(spoken);
    let class = common.class("ds-notification");
    let data = common.data_attributes();
    let plate_id = common.id.clone();
    let open = PressListeners::new(on_open);
    let style = [
        (layers > 0).then(|| format!("--layers:{layers}")),
        swiper.style(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let style = (!style.is_empty()).then(|| style.join(";"));
    rsx! {
        Surface { material, chrome: RootChrome::Transparent,
            div {
                class,
                "data-hover": hover().slug(),
                "data-swipe": swiper.look(),
                style,
                onpointerenter: move |_| point(Hover::Over),
                onpointerleave: move |_| {
                    point(Hover::Away);
                    swiper.left();
                },
                onpointerdown: move |event| swiper.down(&event),
                onpointermove: move |event| swiper.moved(&event),
                onpointerup: move |event| swiper.released(&event),
                onwheel: move |event| swiper.wheel(&event),
                onmounted: move |event| common.mounted(event),
                ..data,
                for n in (1..=layers).rev() {
                    div { key: "{n}", class: "ds-notification-layer", "aria-hidden": "true", style: "--i:{n}" }
                }
                div {
                    class: "ds-notification-plate",
                    id: plate_id,
                    role: "button",
                    tabindex: "0",
                    "aria-label": "{label}",
                    onclick: move |event| {
                        if swiper.click_passes() {
                            open.click(&event);
                        }
                    },
                    onkeydown: move |event| {
                        if opens(&event.key()) {
                            event.prevent_default();
                            on_open.call(Press::primary());
                        }
                    },
                    span { class: "ds-notification-icon",
                        IconView { source: app.icon.clone(), size: IconSize::Px(icon_size) }
                    }
                    div { class: "ds-notification-words",
                        {head(&summary, &age, count)}
                        if let Some(body) = body {
                            NotificationBody { key: "{body.plain_text()}", body: body.clone(), on_link }
                        }
                        if !actions.is_empty() {
                            {action_row(&actions)}
                        }
                    }
                }
                {close_button(on_close)}
            }
        }
    }
}

/// Whether a key on the plate opens the notification: Enter or Space, a button's own keys.
fn opens(key: &Key) -> bool {
    matches!(key, Key::Enter) || *key == Key::Character(" ".into())
}

/// The first line: the summary, the group's count from two, and the age.
fn head(summary: &TextLine, age: &TextLine, count: Option<GroupCount>) -> Element {
    let shown = count.map(|group| group.count).filter(|&count| count > 1);
    rsx! {
        div { class: "ds-notification-head",
            span { class: "ds-notification-summary ds-truncate", {text(summary)} }
            if let Some(count) = shown {
                span { class: "ds-notification-count", "{count}" }
            }
            span { class: "ds-notification-age", {text(age)} }
        }
    }
}

/// The actions, as Mini buttons that keep their press.
fn action_row(actions: &[CardAction]) -> Element {
    rsx! {
        div { class: "ds-notification-actions",
            for (n , action) in actions.iter().cloned().enumerate() {
                Button {
                    key: "{n}",
                    size: ControlSize::Mini,
                    label: action.label,
                    propagation: Propagation::Stop,
                    onclick: move |press| action.on_press.call(press),
                }
            }
        }
    }
}

/// The close button at the top left corner: its own hit target, keeping its press.
fn close_button(on_close: EventHandler<Press>) -> Element {
    let close = PressListeners::new(on_close).with_propagation(Propagation::Stop);
    rsx! {
        button {
            r#type: "button",
            class: "ds-notification-close",
            "aria-label": "Close",
            onclick: move |event| close.click(&event),
            onkeydown: move |event| close.key_down(&event, ActivationKeys::ReturnAndSpace),
            Glyph { icon: Icon::X, size: IconSize::Micro }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::opens;
    use dioxus::prelude::Key;

    #[test]
    fn enter_and_space_open_the_card() {
        let cases = [
            (Key::Enter, true),
            (Key::Character(" ".into()), true),
            (Key::Escape, false),
            (Key::Tab, false),
        ];
        for (key, want) in cases {
            assert_eq!(opens(&key), want, "{key:?}");
        }
    }
}
