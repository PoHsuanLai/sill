//! Pointer input as the dock's handlers see it, and its resolution into machine events against
//! the frame on screen: the hit test, and the drop zone while a tile is pressed or dragged.

use dioxus::html::input_data::MouseButton;
use dioxus::html::point_interaction::{
    InteractionLocation, ModifiersInteraction, PointerInteraction,
};
use dioxus::prelude::{Modifiers, MouseData};
use sill_model::units::Cpx;

use crate::surfaces::dock::frame::Frame;
use crate::surfaces::dock::hit::{drop_zone, hit};
use crate::surfaces::dock::items::DockItem;
use crate::surfaces::dock::machine::{
    Button, DockEvent, DockState, DropZone, Modifier, Point, Press,
};

/// One pointer input, surface-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerInput {
    Enter(Point),
    Move(Point),
    Leave,
    Press(Button, Point),
    Release(Button, Point, Modifier),
}

/// What the session is fed: a machine event as it is, or pointer input to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DockInput {
    Event(DockEvent),
    Pointer(PointerInput),
}

/// `input` as the machine's event, hit-tested against `frame`.
pub(crate) fn resolve(
    input: PointerInput,
    frame: &Frame,
    items: &[DockItem],
    dock: &DockState,
    remove: Cpx,
) -> DockEvent {
    match input {
        PointerInput::Enter(at) => DockEvent::PointerEnter {
            at,
            hit: hit(frame, items, at),
        },
        PointerInput::Move(at) => DockEvent::PointerMove {
            at,
            hit: hit(frame, items, at),
            zone: zone(frame, items, dock, at, remove),
        },
        PointerInput::Leave => DockEvent::PointerLeave,
        PointerInput::Press(button, at) => DockEvent::Press {
            button,
            at,
            hit: hit(frame, items, at),
        },
        PointerInput::Release(button, at, modifier) => DockEvent::Release {
            button,
            at,
            hit: hit(frame, items, at),
            modifier,
        },
    }
}

/// Where the pressed tile would land; no drag, no zone.
fn zone(frame: &Frame, items: &[DockItem], dock: &DockState, at: Point, remove: Cpx) -> DropZone {
    match dock.press {
        Press::Down { tile, .. } | Press::Dragging { tile, .. } => {
            drop_zone(frame, items, tile, at, remove)
        }
        Press::Up => DropZone::Slot(0),
    }
}

/// Where a mouse event happened, surface-local hundredths of a pixel.
pub(crate) fn point(event: &MouseData) -> Point {
    let at = event.client_coordinates();
    Point {
        x: Cpx((at.x * 100.0).round() as i32),
        y: Cpx((at.y * 100.0).round() as i32),
    }
}

/// The button a press or release is about; a button the dock has no use for is none.
pub(crate) fn button(event: &MouseData) -> Option<Button> {
    match event.trigger_button()? {
        MouseButton::Primary => Some(Button::Primary),
        MouseButton::Secondary => Some(Button::Secondary),
        MouseButton::Auxiliary => Some(Button::Middle),
        _ => None,
    }
}

/// design/10 §10.3.6: Super or Ctrl is Cmd, Alt is Option.
pub(crate) fn modifier(event: &MouseData) -> Modifier {
    let held = event.modifiers();
    let cmd = held.intersects(Modifiers::SUPER | Modifiers::CONTROL | Modifiers::META);
    match (cmd, held.contains(Modifiers::ALT)) {
        (true, true) => Modifier::CmdAlt,
        (true, false) => Modifier::Cmd,
        (false, true) => Modifier::Alt,
        (false, false) => Modifier::None,
    }
}
