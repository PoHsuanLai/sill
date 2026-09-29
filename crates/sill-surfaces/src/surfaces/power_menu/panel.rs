//! The power menu's render: one `Ds` root in `power_menu.material` (the active Space's look),
//! as large as the output (`RootExtent::Viewport`), holding quire's `Sheet` centred over its
//! modal scrim with the question, the four action buttons in `SessionAction::POWER_MENU` order
//! (each disabled where the backend's offers say so) and the `Kbd` hints. The sheet is keyed by
//! the opening, so each opening plays the entrance and starts its focus over; it plays its own
//! exit and says when it has settled (`on_hidden`), where the menu closes and a chosen action
//! runs.

use dioxus::prelude::*;
use ds::{
    Availability, BlurState, Button, Ds, Glyph, Icon, IconSize, Kbd, KbdSize, RootExtent,
    ScrimStrength, Sheet, SheetPlacement, Shortcut,
};
use shell_host::OutputInfo;
use sill_model::ids::OutputName;
use sill_services::{Session, SessionAction};

use crate::hooks::{use_appearance, use_ds_scale, use_service_part, use_settings};
use crate::surfaces::launcher::panel::{active_look, material};
use crate::surfaces::power_menu::drive::{MenuCtx, use_menu};
use crate::surfaces::power_menu::keys::offered;
use crate::surfaces::power_menu::labels::{
    action_label, button_size, button_variant, consequence, prompt,
};
use crate::surfaces::power_menu::layer::PANEL_ID;
use crate::surfaces::power_menu::sheet::{MenuSession, SheetInput};
use sill_services::CosmicWlService;

/// The power menu's stylesheet (lint-clean under `Profile::Strict`,
/// `tests/power_menu_lint.rs`).
pub(crate) const POWER_MENU_CSS: &str = include_str!("../../style/power_menu.css");

/// The panel on `output`.
pub(crate) fn power_menu(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(
        &use_service_part::<CosmicWlService, _>(|wl| wl.workspaces.clone())(),
        on.as_ref(),
        &file.spaces,
    );
    let ctx = use_menu(output.id, file.power_menu.default_action);
    let menu = (ctx.menu)();
    let offers = (ctx.offers)();
    rsx! {
        Ds {
            scale: ds_scale,
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
            material: material(file.power_menu.material),
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {POWER_MENU_CSS} }
            if let Some(menu) = menu {
                {sheet(&ctx, menu, &offers)}
            }
        }
    }
}

/// The sheet for one opening.
fn sheet(ctx: &MenuCtx, menu: MenuSession, offers: &Session) -> Element {
    let focused = menu.keys.focused;
    let (close, hidden, keys) = (ctx.clone(), ctx.clone(), ctx.clone());
    rsx! {
        Sheet {
            key: "{menu.serial.0}",
            label: "Power".to_owned(),
            shown: Some(menu.phase.shown()),
            on_hidden: move |()| hidden.input(SheetInput::Settled),
            placement: SheetPlacement::Centre,
            scrim: ScrimStrength::Modal,
            onclose: move |()| close.input(SheetInput::Dismissed),
            div {
                id: PANEL_ID,
                class: "sill-power-menu-panel",
                "data-look": menu.phase.slug(),
                onkeydown: move |event| keys.key(&event),
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
                    {hint(vec![ds::ShortcutKey::Left, ds::ShortcutKey::Right], "Choose")}
                    {hint(vec![ds::ShortcutKey::Enter], action_label(focused))}
                    {hint(vec![ds::ShortcutKey::Escape], "Cancel")}
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
    let variant = button_variant(action, focused);
    rsx! {
        Button {
            key: "{action_label(action)}",
            variant,
            size: button_size(variant),
            label: action_label(action),
            availability,
            mounted: move |event: MountedEvent| mount.mounted(action, event.data()),
            onclick: move |_| press.input(SheetInput::Pressed(action)),
        }
    }
}

/// A key hint: the caps, then what they do.
fn hint(keys: Vec<ds::ShortcutKey>, what: &str) -> Element {
    rsx! {
        span { class: "sill-power-menu-hint",
            Kbd { shortcut: Shortcut(keys), size: KbdSize::Small }
            span { class: "sill-power-menu-hint-text", {what.to_owned()} }
        }
    }
}
