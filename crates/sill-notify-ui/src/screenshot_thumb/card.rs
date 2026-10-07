//! The card: quire's `ShotThumbnail`, the picture letterboxed in a Toast card with its
//! entrance, exit, hover row, swipe to dismiss and drag threshold. The thumbnail feeds it what
//! the machine says (shown or leaving) and hears its presses: a click opens the file, Delete
//! trashes it, a swipe right keeps it (quire's swipe, with the notification keys' metrics, the
//! row carrying the flight), a press crossing the drag threshold carries the file out
//! ([`super::drag`]), and the pointer on it pauses the hold. `THUMB_ID` names the plate, the
//! surface's input and blur region.

use dioxus::prelude::*;
use ds::motion::swipe::SwipeMetrics;
use ds::prelude::*;
use ds::root::common::Common;
use sill_model::screenshots::ShotId;
use sill_shell_kit::notifications::parts::Hover;
use sill_shell_kit::notifications::swipe::NotificationSwipe;
use sill_shell_kit::prelude::*;
use sill_shell_kit::thumbs::shot_press::DragStart;
use sill_shell_kit::thumbs::shot_thumbnail::ThumbAction;

use super::ghost::GhostLook;
use super::io::Thumb;
use super::layer::{THUMB_ID, THUMB_WIDTH};
use super::picture::ThumbPicture;
use super::step::{ThumbExit, ThumbIn};

/// One screenshot's card, `shown` or playing its exit.
#[component]
pub(super) fn ThumbCard(
    id: ShotId,
    picture: ThumbPicture,
    shown: Shown,
    metrics: SwipeMetrics,
    look: GhostLook,
) -> Element {
    let thumb = use_context::<Thumb>();
    let delete = ThumbAction {
        icon: Icon::Trash,
        label: "Delete".into(),
        onpress: EventHandler::new(move |()| thumb.input(ThumbIn::TrashPressed)),
    };
    let dragged = picture.clone();
    rsx! {
        ShotThumbnail {
            image: picture.image.clone(),
            size: picture.size,
            shown,
            on_hidden: move |()| thumb.settled(id, ThumbExit::Card),
            width: Px(THUMB_WIDTH as f32),
            actions: vec![delete],
            onopen: move |()| thumb.input(ThumbIn::Click),
            ondrag: move |_: DragStart| thumb.drag_out(id, &dragged, look.clone()),
            onhover: move |hover: Hover| thumb.input(match hover {
                Hover::Over => ThumbIn::PointerEnter,
                Hover::Away => ThumbIn::PointerLeave,
            }),
            common: Common { id: Some(THUMB_ID.to_owned()), ..Common::default() },
            swipe: NotificationSwipe::Dismiss(EventHandler::new(move |()| thumb.input(ThumbIn::SwipedAway))),
            swipe_metrics: metrics,
        }
    }
}
