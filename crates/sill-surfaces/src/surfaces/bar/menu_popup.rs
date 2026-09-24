//! The bar popup's own document: a `Ds` root holding one ds
//! `Menu{Dropdown}` (or, for the volume item, a ds `Popover` with a `Slider` and a `Toggle`),
//! following the bar's view channel: a new view swaps the contents and repositions the popup
//! (`xdg_popup.reposition`), none closes it.

use dioxus::prelude::*;
use ds::{
    Align, Anchor, BlurState, Ds, Elevation, Fraction, HeaderKind, Material, Menu, MenuKind,
    Placement, Point, Popover, Px, SectionHeader, Side, Slider, Switch, Toggle,
};
use shell_host::{PopupPlacement, SurfaceOp, use_surface, use_watch};
use sill_services::{Control, Mute, Percent, VolumeCommand};
use tokio::sync::{mpsc, watch};

use super::content::{PopupBody, VolumePanel};
use super::geometry::{MARGIN, frame_style};
use super::pick::BarPick;
use super::plan::PopupReport;
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

/// The popup root.
#[component]
pub(crate) fn MenuPopup() -> Element {
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
    };
    rsx! {
        style { {POPUP_CSS} }
        div { class: "sill-popup-frame", style: frame_style(),
            // Not `Material::Popover` (design/03 §17.3): the root holds only overlays, so it is
            // a zero-height, frame-wide box, and Popover's drop shadow on it paints a grey band
            // across the popup's top (FINDINGS "M1 bar-ui", Q13). Bar's material has no drop
            // shadow; the menu card paints itself (`.ds-popover`) whatever the root's material.
            Ds {
                appearance: look.appearance,
                system: look.system,
                material: Material::Bar,
                blur: BlurState::Unavailable,
                tint_alpha: Some(look.tint),
                {content}
            }
        }
    }
}

/// A ds `Menu{Dropdown}`; the root keys it by the bar menu so a hover switch starts a fresh
/// selection.
///
/// The ds `Menu` calls `onclose` and then `onpick` for a pick, so a close is reported only
/// after the handler has returned, and only when no pick followed it.
#[component]
fn BarMenuList(entries: Vec<ds::MenuEntry<BarPick>>) -> Element {
    let link = use_context::<PopupLink>();
    let mut picked = use_hook(|| CopyValue::new(false));
    let on_pick = {
        let reports = link.reports.clone();
        move |pick: BarPick| {
            picked.set(true);
            let _ = reports.send(PopupReport::Picked(pick));
        }
    };
    let on_close = {
        let reports = link.reports.clone();
        move |()| {
            let reports = reports.clone();
            spawn(async move {
                if !picked() {
                    let _ = reports.send(PopupReport::Dismissed);
                }
            });
        }
    };
    rsx! {
        Menu {
            kind: MenuKind::Dropdown,
            anchor: corner(),
            entries,
            onpick: on_pick,
            onclose: on_close,
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
