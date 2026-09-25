//! The card (quire gap Q181 until quire has a thumbnail card): the picture letterboxed in a
//! Toast `Surface`, `THUMB_WIDTH` wide at 16:10, and a hover row with Delete. A click opens the
//! file, quire's swipe (`use_swipe`, with the notification keys' metrics) dismisses and keeps
//! it, a press moved left or up past the same threshold is a drag out ([`super::gesture`]), and
//! the pointer on it pauses the hold. The card element is `THUMB_ID`, the surface's input and
//! blur region.

use std::path::Path;

use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use ds::{Click, Held, Icon, IconButton, IconButtonVariant, IconUrl, Material, Point, Propagation};
use ds::{Corner, Px, Radius, Surface, SwipeInput, SwipeMetrics, Swiper, use_swipe};
use sill_services::{Shot, ShotPreview};

use super::drag::{DragStart, start_file_drag};
use super::drive::Thumb;
use super::gesture::{DragOut, Gesture, GestureIn, gesture_step};
use super::layer::{THUMB_ID, THUMB_WIDTH};
use super::machine::{Pointer, ThumbIn};

/// The card's height: a 16:10 picture's at `THUMB_WIDTH` (a taller one is letterboxed).
const THUMB_HEIGHT: u32 = THUMB_WIDTH * 10 / 16;

/// Pixels per line of a line-based wheel delta (what Blitz scrolls a line by, as quire's card).
const LINE_PX: f64 = 20.0;

/// One screenshot's card.
#[component]
pub(super) fn ThumbCard(shot: Shot, metrics: SwipeMetrics) -> Element {
    let thumb = use_context::<Thumb>();
    let swiper = use_swipe(
        metrics,
        EventHandler::new(move |()| thumb.input(ThumbIn::SwipedAway)),
    );
    let gesture = use_hook(|| CopyValue::new(Gesture::Idle));
    let press = Press {
        thumb,
        swiper,
        gesture,
        threshold: metrics.dismiss,
    };
    let file = shot.path.clone();
    let state = swiper.state();
    let style = format!(
        "width:{THUMB_WIDTH}px;height:{THUMB_HEIGHT}px;transform:translateX({}px);",
        state.offset().0
    );
    let pointer = match thumb.pointer() {
        Pointer::Over => "over",
        Pointer::Away => "away",
    };
    rsx! {
        div {
            class: "sill-thumb",
            id: THUMB_ID,
            "data-swipe": state.look().slug(),
            "data-pointer": pointer,
            style,
            onpointerenter: move |_| thumb.input(ThumbIn::PointerEnter),
            onpointerleave: move |_| {
                thumb.input(ThumbIn::PointerLeave);
                press.up();
            },
            onpointerdown: move |event| press.down(&event),
            onpointermove: {
                let file = file.clone();
                move |event: PointerEvent| press.moved(&event, &file)
            },
            onpointerup: move |_| press.up(),
            onwheel: move |event| press.wheel(&event),
            onclick: move |_| {
                if swiper.take_click() == Click::Passes {
                    thumb.input(ThumbIn::Click);
                }
            },
            Surface { material: Material::Toast,
                div { class: "sill-thumb-picture",
                    if let Some(url) = picture_url(&shot) {
                        img { class: "sill-thumb-image", src: "{url}", alt: "Screenshot" }
                    }
                }
            }
            div { class: "sill-thumb-actions",
                Surface { material: Material::Toast, radius: Some(Corner::Token(Radius::Pill)),
                    IconButton {
                    variant: IconButtonVariant::Tool,
                    icon: Icon::Trash,
                    label: "Delete".to_owned(),
                    tooltip: Some("Move to Trash".to_owned()),
                    propagation: Propagation::Stop,
                    onclick: move |_| thumb.input(ThumbIn::TrashPressed),
                    }
                }
            }
        }
    }
}

/// The picture a card draws: the preview, or the file itself when none could be made.
fn picture_url(shot: &Shot) -> Option<String> {
    let path = match &shot.preview {
        ShotPreview::Ready(preview) => preview.as_path(),
        ShotPreview::Original => shot.path.as_path(),
        ShotPreview::Pending => return None,
    };
    IconUrl::file(path).ok().map(|url| url.as_str().to_owned())
}

/// The card's pointer handlers' shared parts.
#[derive(Clone, Copy)]
struct Press {
    thumb: Thumb,
    swiper: Swiper,
    gesture: CopyValue<Gesture>,
    threshold: Px,
}

impl Press {
    fn down(self, event: &PointerEvent) {
        let at = point(event);
        if event.trigger_button() == Some(MouseButton::Primary) {
            self.swiper.feed(SwipeInput::Down {
                x: at.x,
                at: self.swiper.now(),
            });
            self.gesture_input(GestureIn::Down(at));
        }
    }

    fn moved(self, event: &PointerEvent, file: &Path) {
        let at = point(event);
        let held = match event.held_buttons().contains(MouseButton::Primary) {
            true => Held::Primary,
            false => Held::Nothing,
        };
        self.swiper.pointer_moved(at.x, held);
        let input = match held {
            Held::Primary => GestureIn::Move(at),
            Held::Nothing => GestureIn::Up,
        };
        if self.gesture_input(input) == DragOut::Start {
            self.drag_out(file);
        }
    }

    fn up(self) {
        self.swiper.feed(SwipeInput::Up {
            at: self.swiper.now(),
        });
        self.gesture_input(GestureIn::Up);
    }

    fn wheel(self, event: &WheelEvent) {
        let (dx, dy) = match event.delta() {
            WheelDelta::Pixels(v) => (v.x, v.y),
            WheelDelta::Lines(v) => (v.x * LINE_PX, v.y * LINE_PX),
            WheelDelta::Pages(_) => (0.0, 0.0),
        };
        self.swiper.feed(SwipeInput::Scroll {
            dx: Px(dx as f32),
            dy: Px(dy as f32),
        });
    }

    fn gesture_input(self, input: GestureIn) -> DragOut {
        let mut gesture = self.gesture;
        let (next, out) = gesture_step(*gesture.peek(), input, self.threshold);
        gesture.set(next);
        out
    }

    /// Hand the file over; with no drag source yet (G120) the card stays where it is.
    fn drag_out(self, file: &Path) {
        match start_file_drag(file) {
            DragStart::Started => self.thumb.input(ThumbIn::DraggedOut),
            DragStart::Unsupported => {
                eprintln!(
                    "screenshot_thumb: no drag out until shell-host has a drag source (G120)"
                );
            }
        }
    }
}

fn point(event: &PointerEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}
