//! The bar popup's own document: a `Ds` root holding one ds
//! `Menu{Dropdown}` (or, for the volume item, a ds `Popover` with a `Slider` and a `Toggle`),
//! following the bar's view channel: a new view swaps the contents and repositions the popup
//! (`xdg_popup.reposition`), none closes it. The root is `Material::Popover`, which paints
//! nothing on its own box (quire's transparent popup root); the menu card paints the material.
//! The menu opens with no entrance (design/13 §13.3.2) and reports the pointer's hover and
//! release to the bar's tracking machine.

use dioxus::prelude::*;
use ds::{
    Align, Anchor, BlurState, Ds, Elevation, Fraction, HeaderKind, Material, Menu, MenuEntrance,
    MenuKind, Placement, Point, Popover, Press, Px, SectionHeader, Side, Slider, Switch, Toggle,
};
use shell_host::{PopupPlacement, SurfaceOp, use_surface, use_watch};
use sill_services::{Control, Mute, Percent, VolumeCommand};
use tokio::sync::{mpsc, watch};

use super::content::{PopupBody, VolumePanel};
use super::geometry::{MARGIN, frame_style};
use super::pick::BarPick;
use super::plan::{PopupReport, choice};
use super::session::PopupView;

/// The popup's link to its bar, provided as its root context.
#[derive(Debug, Clone)]
pub(crate) struct PopupLink {
    pub view: watch::Receiver<Option<PopupView>>,
    pub reports: mpsc::UnboundedSender<PopupReport>,
    pub volume: Control<VolumeCommand>,
}

/// The anchor that puts a ds floating surface `MARGIN` inside the frame's top-left corner, so at
/// (`SIDE`, `TOP`) in the popup: `place` clamps it there whatever its size, since the frame is
/// the surface plus the margin.
fn corner() -> Anchor {
    let at = f32::from(u8::try_from(MARGIN).unwrap_or(8));
    Anchor::Point(Point {
        x: Px(at),
        y: Px(1.0),
    })
}

/// The popup document's own stylesheet: the frame its `Ds` root sits in.
pub(crate) const POPUP_CSS: &str = include_str!("../../style/bar_popup.css");

/// The gap from [`corner`]'s point to the menu's top edge.
const GAP: Px = Px(7.0);

/// The popup root. A popup is a `VirtualDom` of its own and inherits nothing from the bar, so
/// it provides quire's rect measurer itself.
#[component]
pub(crate) fn MenuPopup() -> Element {
    let ds_scale = crate::hooks::use_ds_scale();
    ds_native::measure::provide();
    let link = use_context::<PopupLink>();
    let surface = use_surface();
    let view = use_watch(link.view.clone());
    let mut placed = use_hook(|| CopyValue::new(None::<PopupPlacement>));
    let reports = link.reports.clone();
    use_drop(move || {
        // The bar may be gone first; then there is nobody to tell.
        let _ = reports.send(PopupReport::Gone);
    });
    use_effect(move || match view() {
        None => surface.close(),
        Some(next) => {
            let before = placed.replace(Some(next.placement));
            if before.is_some_and(|before| before != next.placement) {
                surface.apply(SurfaceOp::Reposition(next.placement));
            }
        }
    });
    let Some(current) = view() else {
        return rsx! {};
    };
    let look = current.look;
    let which = current.menu;
    let content = match current.body {
        PopupBody::Menu(entries) => rsx! {
            BarMenuList { key: "{which:?}", entries }
        },
        PopupBody::Volume(panel) => rsx! {
            VolumeMenu { panel }
        },
        // M5 agent (c): `control_center::ControlCenter` inside a quire `Popover`, with the root
        // contexts it needs and a report for its page changes (FINDINGS "M5 freeze").
        PopupBody::ControlCenter(_) => todo!("M5 fill: control-center (c)"),
    };
    rsx! {
        style { {POPUP_CSS} }
        div {
            class: "sill-popup-frame",
            style: "{frame_style()};{look.metrics.style_attr()}",
            Ds {
                scale: ds_scale,
                appearance: look.appearance,
                system: look.system,
                material: Material::Popover,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                stack: Some(look.stack),
                {content}
            }
        }
    }
}

/// A ds `Menu{Dropdown}`, at once (no entrance: a bar menu, and every hover switch); the root
/// keys it by the bar menu so a switch starts a fresh selection. Its hover and releases go to
/// the tracking machine, which decides a press-drag-release; quire's own pick then finds the
/// session closed. The keyboard's pick still arrives as `Picked`, and quire calls `onpick`
/// before `onclose`, so a close reports a dismissal only when nothing was picked.
#[component]
fn BarMenuList(entries: Vec<ds::MenuEntry<BarPick>>) -> Element {
    let link = use_context::<PopupLink>();
    let mut picked = use_hook(|| CopyValue::new(false));
    let mut hovered = use_hook(|| CopyValue::new(None::<usize>));
    let send = {
        let reports = link.reports.clone();
        move |report: PopupReport| {
            // The bar may be gone first; then there is nobody to tell.
            let _ = reports.send(report);
        }
    };
    let on_pick = {
        let send = send.clone();
        move |pick: BarPick| {
            picked.set(true);
            send(PopupReport::Picked(pick));
        }
    };
    let on_close = {
        let send = send.clone();
        move |()| {
            if !picked() {
                send(PopupReport::Dismissed);
            }
        }
    };
    let on_hover = {
        let send = send.clone();
        move |index: Option<usize>| {
            hovered.set(index);
            send(PopupReport::Hovered(index));
        }
    };
    let on_release = {
        let entries = entries.clone();
        move |_: Press| {
            let over = hovered().and_then(|index| choice(&entries, index));
            send(PopupReport::Released(over));
        }
    };
    rsx! {
        Menu {
            kind: MenuKind::Dropdown,
            anchor: corner(),
            entries,
            onpick: on_pick,
            onclose: on_close,
            on_hover,
            on_release,
            entrance: MenuEntrance::Instant,
        }
    }
}

/// The volume panel: the device and level as a field header, a level slider, and mute.
#[component]
fn VolumeMenu(panel: VolumePanel) -> Element {
    let link = use_context::<PopupLink>();
    let level = link.volume.clone();
    let mute = link.volume.clone();
    let reports = link.reports.clone();
    let percent = panel.level.0 / 10;
    let muted = match panel.mute {
        Mute::Muted => Switch::On,
        Mute::Audible => Switch::Off,
    };
    rsx! {
        Popover {
            anchor: corner(),
            placement: Placement::new(Side::Bottom, Align::Start),
            gap: GAP,
            elevation: Elevation::Pop,
            onclose: move |()| {
                let _ = reports.send(PopupReport::Dismissed);
            },
            div { class: "sill-volume-panel",
                SectionHeader {
                    kind: HeaderKind::Field,
                    text: panel.heading.clone(),
                    value: Some(format!("{percent}%")),
                }
                Slider {
                    label: "Volume".to_owned(),
                    value: panel.level,
                    step: Fraction(50),
                    onchange: move |value: Fraction| {
                        let _ = level.send(VolumeCommand::SetLevel(Percent(value.0 / 10)));
                    },
                }
                Toggle {
                    label: "Mute".to_owned(),
                    value: muted,
                    onchange: move |value: Switch| {
                        let to = match value {
                            Switch::On => Mute::Muted,
                            Switch::Off => Mute::Audible,
                        };
                        let _ = mute.send(VolumeCommand::SetMute(to));
                    },
                }
            }
        }
    }
}
