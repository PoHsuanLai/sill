//! ShotThumbnail: the floating thumbnail after a screenshot (design/04-COMPONENTS.md section 39,
//! design/20 section 1.13).
//!
//! The picture sits letterboxed in a card of the Toast material ([`shot_frame`]), inside a
//! transparent Toast scope, so the card paints the material wherever its root is (as
//! `NotificationCard` does). Showing and hiding are the caller's (`shown`), since it owns the
//! hold (`screenshot.thumbnail_hold_ms`) and knows when a dismissal happened; the card slides in
//! (`Anim::PanelIn`), slides out to the right (`Anim::PanelOut`) and calls `on_hidden` once that
//! exit has settled, so the host can unmap the surface. The pointer's arrival and departure go
//! to `onhover`, so the caller's hold can pause while the card is looked at, and show the
//! actions. A press on the picture opens it on its click, or, once it has travelled
//! [`crate::DRAG_THRESHOLD`], asks the host to start a drag ([`DragStart`]) and does not open.
//!
//! `swipe: Swipe::Dismiss(..)` lets a drag or a horizontal scroll to the right dismiss the card
//! exactly as it dismisses a `NotificationCard` (`notification_swipe`, with the same
//! `notifications.swipe_*` metrics): alone it flies out itself and reports at the flight's
//! settle; in a `BannerStack` row the row carries the flight. A press heading mostly right is
//! then the swipe's and never a drag out. The card places nothing itself (no margin, no
//! anchor), so a `BannerStack` or the caller's surface decides where it sits.

use crate::notifications::parts::Hover;
use crate::notifications::swipe::{NotificationSwipe, use_card_swipe};
use crate::thumbs::shot_frame::shot_frame;
use crate::thumbs::shot_press::{DragLane, DragStart, PressInput, ShotPress};
use dioxus::prelude::*;
use ds::base::geometry::units::{Point, Px};
use ds::base::vocab::Shown;
use ds::base::word::Word;
use ds::components::content::image_source::{ImageSize, ImageSource};
use ds::components::content::picture_fit::picture_style;
use ds::components::content::text_runs::TextLine;
use ds::components::controls::button_model::{Bezel, ImagePosition};
use ds::components::controls::press::Propagation;
use ds::components::overlays::swipe_glue::SwipeGlue;
use ds::motion::anim::Anim;
use ds::motion::presence::spec::PresenceSpec;
use ds::motion::presence::use_presence::{EntranceAlias, Presented, use_presence};
use ds::motion::presence::{Exit, Presence};
use ds::motion::swipe::SwipeMetrics;
use ds::prelude::*;
use ds::root::chrome::RootChrome;
use ds::root::common::Common;
use ds::root::surface::Surface;
use ds::style::appearance::material::Material;
use ds::style::icon::Icon;

/// One action the thumbnail offers on hover: Delete now; Mark Up and Copy Text once the shell
/// has them. Its press stays at its button: it never also opens the picture.
#[derive(Debug, Clone, PartialEq)]
pub struct ThumbAction {
    /// Its glyph.
    pub icon: Icon,
    /// Its name, read as the button's label.
    pub label: TextLine,
    /// What a press does.
    pub onpress: EventHandler<()>,
}

/// The screenshot thumbnail. `image` is drawn at `width` (240 by default, sill's surface) in a
/// box of `size`'s ratio held between 2:1 and 16:10. `common.id` names the card's plate for the
/// host's input and blur region (`Element("thumb")`); the consumer's classes and data go on its
/// root. `swipe` turns swipe to dismiss on, with
/// `swipe_metrics` from the `notifications.swipe_*` keys.
#[component]
pub fn ShotThumbnail(
    image: ImageSource,
    size: ImageSize,
    shown: Shown,
    #[props(default)] on_hidden: EventHandler<()>,
    #[props(default = Px(240.0))] width: Px,
    #[props(default)] actions: Vec<ThumbAction>,
    #[props(default)] onopen: Option<EventHandler<()>>,
    #[props(default)] ondrag: Option<EventHandler<DragStart>>,
    #[props(default)] onhover: Option<EventHandler<Hover>>,
    #[props(default)] common: Common,
    #[props(default)] swipe: NotificationSwipe,
    #[props(default)] swipe_metrics: SwipeMetrics,
) -> Element {
    let spec = PresenceSpec {
        enter: Anim::PanelIn,
        exit: Exit::PanelOut,
    };
    let Presented { presence, alias } = use_presence(shown, spec, Some(on_hidden));
    rsx! {
        Surface { material: Material::Toast, chrome: RootChrome::Transparent,
            // Keyed by the entrance's alias, which flips on each fresh showing: a card shown
            // again after a swipe flew it out starts with a new swipe (the machine's `Gone` is
            // final, as a notification is dropped after it), a new press and no hover.
            ShotCard {
                key: "{alias.slug()}",
                presence,
                alias,
                image,
                size,
                width,
                actions,
                onopen,
                ondrag,
                onhover,
                common,
                swipe,
                swipe_metrics,
            }
        }
    }
}

/// One showing's card: its swipe, its press and its hover.
#[component]
fn ShotCard(
    presence: Presence,
    alias: EntranceAlias,
    image: ImageSource,
    size: ImageSize,
    width: Px,
    actions: Vec<ThumbAction>,
    onopen: Option<EventHandler<()>>,
    ondrag: Option<EventHandler<DragStart>>,
    onhover: Option<EventHandler<Hover>>,
    common: Common,
    swipe: NotificationSwipe,
    swipe_metrics: SwipeMetrics,
) -> Element {
    let lane = match swipe {
        NotificationSwipe::Dismiss(_) => DragLane::NotRight,
        NotificationSwipe::Off => DragLane::Any,
    };
    let swiper = use_card_swipe(&swipe, swipe_metrics);
    let press = use_signal(ShotPress::default);
    let mut hover = use_signal(Hover::default);
    let mut point = move |next: Hover| {
        if *hover.peek() != next {
            hover.set(next);
            if let Some(onhover) = onhover {
                onhover.call(next);
            }
        }
    };
    let frame = shot_frame(width, size);
    let class = common.class("ds-shot");
    let data = common.data_attributes();
    let plate_id = common.id.clone();
    rsx! {
        div {
            class,
            "data-shown": presence.shown().slug(),
            "data-presence": presence.drawn_slug(),
            "data-pulse": alias.slug(),
            "data-hover": hover().slug(),
            "data-swipe": swiper.look(),
            style: card_style(frame.card.width, swiper.style()),
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
            div {
                class: "ds-shot-plate",
                id: plate_id,
                role: "group",
                "aria-label": "Screenshot",
                style: "height:{frame.card.height.0}px",
                {picture(image, picture_style(frame.picture), Press { press, swiper, lane, onopen, ondrag })}
                if !actions.is_empty() {
                    {action_row(actions)}
                }
            }
        }
    }
}

/// The card's inline width, and its swipe offset while it is off its place.
fn card_style(width: Px, swipe: Option<String>) -> String {
    match swipe {
        Some(offset) => format!("width:{}px;{offset}", width.0),
        None => format!("width:{}px", width.0),
    }
}

/// What the picture's press needs: its machine, the card's swipe (whose drag must not also open),
/// the lane a drag out may take, and who hears the open and the drag.
#[derive(Clone, Copy)]
struct Press {
    press: Signal<ShotPress>,
    swiper: SwipeGlue,
    lane: DragLane,
    onopen: Option<EventHandler<()>>,
    ondrag: Option<EventHandler<DragStart>>,
}

impl Press {
    fn feed(self, input: PressInput) {
        let mut press = self.press;
        let (next, start) = press.peek().clone().step(input);
        press.set(next);
        if let (Some(start), Some(ondrag)) = (start, self.ondrag) {
            ondrag.call(start);
        }
    }

    fn open(self) {
        if let Some(onopen) = self.onopen {
            onopen.call(());
        }
    }

    /// A click opens only when neither a drag out nor a swipe ended with it.
    fn click(self) {
        let swiped = !self.swiper.click_passes();
        if self.press.peek().opens() && !swiped {
            self.open();
        }
    }
}

/// The picture and its press target.
fn picture(image: ImageSource, style: String, press: Press) -> Element {
    rsx! {
        div {
            class: "ds-shot-picture",
            role: "button",
            tabindex: "0",
            "aria-label": "Open screenshot",
            onpointerdown: move |event| press.feed(PressInput::Down(client(&event))),
            onpointermove: move |event| press.feed(PressInput::Moved(client(&event), press.lane)),
            onpointerup: move |_| press.feed(PressInput::Up),
            onpointercancel: move |_| press.feed(PressInput::Up),
            onclick: move |_| press.click(),
            onkeydown: move |event| {
                if matches!(event.key(), Key::Enter) || event.key() == Key::Character(" ".into()) {
                    event.prevent_default();
                    press.open();
                }
            },
            img { class: "ds-shot-image", alt: "", src: image.0, style, draggable: "false" }
        }
    }
}

/// A pointer event's client point.
fn client(event: &PointerEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}

/// The hover row: strip buttons, each keeping its press.
fn action_row(actions: Vec<ThumbAction>) -> Element {
    rsx! {
        div { class: "ds-shot-actions", role: "toolbar", "aria-label": "Screenshot actions",
            for (j , action) in actions.into_iter().enumerate() {
                span { key: "{j}", class: "ds-shot-action",
                    Button {
                        bezel: Bezel::Toolbar, image: ImagePosition::Only,
                        icon: action.icon,
                        label: action.label.plain_text(),
                        propagation: Propagation::Stop,
                        onclick: move |_| action.onpress.call(()),
                    }
                }
            }
        }
    }
}
