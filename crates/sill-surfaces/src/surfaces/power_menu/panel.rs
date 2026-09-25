//! The power menu's render: a transparent `Ds` root (the active Space's look; it paints
//! nothing and gives the exit fade its motion tokens) holding the room, which fades out as a
//! whole; in it, a `Ds` in `power_menu.material` holds quire's `Sheet` (its own full-output
//! scrim, `peek-in`) with the question, the four action buttons in `SessionAction::POWER_MENU`
//! order (each unavailable where the backend's offers say so) and the `Kbd` hints. The sheet
//! is keyed by the opening, so each opening plays the entrance and starts its focus over.

use dioxus::prelude::*;
use ds::{
    Availability, BlurState, Button, Ds, Glyph, Icon, IconSize, Inject, Kbd, KbdSize, Material,
    RootChrome, Sheet, Shortcut,
};
use shell_host::OutputInfo;
use sill_services::{OutputName, Session, SessionAction};

use crate::hooks::{use_appearance, use_ds_scale, use_settings, use_workspaces};
use crate::surfaces::launcher::panel::{active_look, material};
use crate::surfaces::power_menu::drive::{MenuCtx, use_menu};
use crate::surfaces::power_menu::keys::offered;
use crate::surfaces::power_menu::labels::{action_label, button_variant, consequence, prompt};
use crate::surfaces::power_menu::layer::PANEL_ID;
use crate::surfaces::power_menu::sheet::{MenuSession, SheetInput};

/// The power menu's stylesheet (lint-clean under `Profile::Strict`,
/// `tests/power_menu_lint.rs`).
pub(crate) const POWER_MENU_CSS: &str = include_str!("../../style/power_menu.css");

/// The panel on `output`.
pub(crate) fn power_menu(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: Material::Sheet,
            chrome: Some(RootChrome::Transparent),
            blur: BlurState::Available,
            style { {POWER_MENU_CSS} }
            Room { output }
        }
    }
}

/// The whole output: the fading box around the sheet's own root.
#[component]
fn Room(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let ctx = use_menu(output.id, file.power_menu.default_action);
    let menu = (ctx.menu)();
    let offers = (ctx.offers)();
    let keys = ctx.clone();
    rsx! {
        div {
            class: "sill-power-menu",
            "data-look": menu.map_or("closed", |m| m.phase.slug()),
            onkeydown: move |event| keys.key(&event),
            Ds {
                scale: ds_scale,
                appearance: environment.settings.appearance.appearance(),
                system: environment.system,
                look,
                material: material(file.power_menu.material),
                blur: BlurState::Available,
                stylesheet: Inject::Host,
                tint_alpha: Some(environment.tint_alpha()),
                stack: Some(environment.material_stack()),
                // Gives this root the output's height, so the overlay the sheet floats in (and
                // its scrim) covers the output: a `.ds` root is as tall as its content.
                div { class: "sill-power-menu-floor" }
                if let Some(menu) = menu {
                    {sheet(&ctx, menu, &offers)}
                }
            }
        }
    }
}

/// The sheet for one opening.
fn sheet(ctx: &MenuCtx, menu: MenuSession, offers: &Session) -> Element {
    let focused = menu.keys.focused;
    let close = ctx.clone();
    rsx! {
        Sheet {
            key: "{menu.serial.0}",
            label: "Power".to_owned(),
            onclose: move |()| close.input(SheetInput::Dismissed),
            div { id: PANEL_ID, class: "sill-power-menu-panel",
                div { class: "sill-power-menu-mark",
                    Glyph { icon: Icon::Power, size: IconSize::Bar }
                }
                div { class: "sill-power-menu-prompt", {prompt(focused)} }
                div { class: "sill-power-menu-detail", {consequence(focused)} }
                div { class: "sill-power-menu-buttons",
                    for action in SessionAction::POWER_MENU {
                        {button(ctx, action, focused, offers)}
                    }
                }
                div { class: "sill-power-menu-hints",
                    {hint(vec![ds::Key::Left, ds::Key::Right], "Choose")}
                    {hint(vec![ds::Key::Enter], action_label(focused))}
                    {hint(vec![ds::Key::Escape], "Cancel")}
                }
            }
        }
    }
}

/// One action's button.
fn button(
    ctx: &MenuCtx,
    action: SessionAction,
    focused: SessionAction,
    offers: &Session,
) -> Element {
    let (mount, press) = (ctx.clone(), ctx.clone());
    let availability = if offered(offers, action) {
        Availability::Enabled
    } else {
        Availability::Disabled
    };
    rsx! {
        Button {
            key: "{action_label(action)}",
            variant: button_variant(action, focused),
            label: action_label(action),
            availability,
            mounted: move |event: MountedEvent| mount.mounted(action, event.data()),
            onclick: move |_| press.input(SheetInput::Pressed(action)),
        }
    }
}

/// A key hint: the caps, then what they do.
fn hint(keys: Vec<ds::Key>, what: &str) -> Element {
    rsx! {
        span { class: "sill-power-menu-hint",
            Kbd { shortcut: Shortcut(keys), size: KbdSize::Small }
            span { class: "sill-power-menu-hint-text", {what.to_owned()} }
        }
    }
}
