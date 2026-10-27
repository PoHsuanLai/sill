//! The power menu's render: one `Ds` root in the sheet material (the active Space's look),
//! as large as the output (`RootExtent::Viewport`), holding quire's `Alert` centred on its `Sheet`
//! (it dims nothing), as macOS's shut-down dialog: the question about the opening's action, Cancel
//! (Escape presses it) and that action's button (Return presses it), side by side. An action that
//! ends the apps counts down under the question and carries itself out when the count runs out. The alert is keyed by the opening, so each opening plays the
//! entrance and starts its focus over; it plays its own exit and says when it has settled
//! (`on_hidden`), where the menu closes and a chosen action runs.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole};
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::session::Session;

use super::choice::answers;
use super::io::{MenuCtx, use_menu};
use super::labels::{NONE_OFFERED_MESSAGE, NONE_OFFERED_TITLE, action_label, consequence, prompt};
use super::sheet::{MenuSession, SheetInput};
use super::surface::PANEL_ID;
use ds::base::time::clock::sleep;
use sill_model::session::words::{COUNTDOWN_SECONDS, countdown};
use sill_ui_kit::links::OpenSerial;
use sill_ui_kit::root::{ClickAway, SurfaceRoot};
use sill_ui_kit::sheet_stage::SheetExit;
use std::time::Duration;

/// The power menu's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const POWER_MENU_CSS: &str = include_str!("style.css");

/// The panel on `output`.
pub(super) fn power_menu(output: OutputInfo) -> Element {
    let ctx = use_menu(output.id);
    let menu = (ctx.menu)();
    let offers = (ctx.offers)();
    let left = use_countdown(&ctx);
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: POWER_MENU_CSS }
            if let Some(menu) = menu {
                {alert(&ctx, menu, &offers, left)}
            }
        }
    }
}

/// The alert for one opening.
fn alert(ctx: &MenuCtx, menu: MenuSession, offers: &Session, left: u32) -> Element {
    let (hidden, catch) = (ctx.clone(), ctx.clone());
    let buttons = buttons(ctx, menu, offers);
    let (title, message) = if answers(menu.default, offers).is_empty() {
        (NONE_OFFERED_TITLE, NONE_OFFERED_MESSAGE.to_owned())
    } else {
        (
            prompt(menu.default),
            countdown(menu.default, left).unwrap_or_else(|| consequence(menu.default).to_owned()),
        )
    };
    rsx! {
        Fragment { key: "{menu.serial.0}",
            // A sheet dims nothing and catches no click (macOS; design/30): a press outside it is
            // this surface's own, so it closes the menu as the scrim did.
            ClickAway { onpress: move |()| catch.input(SheetInput::Dismissed) }
            Alert {
                title,
                message: Some(TextLine::from(message)),
                icon: Icon::Power,
                buttons,
                shown: Some(menu.phase.shown()),
                on_hidden: move |()| hidden.input(SheetInput::Settled),
                common: Common {
                    id: Some(PANEL_ID.to_owned()),
                    ..Common::default()
                },
            }
        }
    }
}

/// The alert's answers, as macOS's: Cancel, then the opening's action, the default on the right.
fn buttons(ctx: &MenuCtx, menu: MenuSession, offers: &Session) -> Vec<AlertButton> {
    let action = answers(menu.default, offers).into_iter().map(|action| {
        let press = ctx.clone();
        AlertButton::new(
            action_label(action),
            AlertRole::Normal,
            EventHandler::new(move |()| press.input(SheetInput::Pressed(action))),
        )
    });
    let cancel = ctx.clone();
    let cancel = AlertButton::new(
        "Cancel",
        AlertRole::Cancel,
        EventHandler::new(move |()| cancel.input(SheetInput::Dismissed)),
    );
    action.chain(std::iter::once(cancel)).collect()
}

/// The seconds left on the count of the open alert: a second at a time while it is up, and its
/// action pressed when the count runs out. Each opening starts its own count; a closing or a
/// leaving alert stops it. Only an action that asks first counts (Lock and Sleep carry themselves
/// out), and only when logind offers it.
fn use_countdown(ctx: &MenuCtx) -> u32 {
    let mut left = use_signal(|| COUNTDOWN_SECONDS);
    let mut running = use_hook(|| CopyValue::new(None::<(OpenSerial, dioxus::core::Task)>));
    let ctx = ctx.clone();
    use_effect(move || {
        let menu = (ctx.menu)();
        let offers = *ctx.offers.peek();
        let wanted = menu.filter(|menu| {
            menu.phase == SheetExit::Open
                && !answers(menu.default, &offers).is_empty()
                && countdown(menu.default, COUNTDOWN_SECONDS).is_some()
        });
        let now = *running.peek();
        match (wanted, now) {
            (Some(menu), Some((serial, _))) if serial == menu.serial => {}
            (wanted, now) => {
                if let Some((_, task)) = now {
                    task.cancel();
                }
                running.set(None);
                if let Some(menu) = wanted {
                    left.set(COUNTDOWN_SECONDS);
                    let press = ctx.clone();
                    let task = spawn(async move {
                        for remaining in (0..COUNTDOWN_SECONDS).rev() {
                            sleep(Duration::from_secs(1)).await;
                            left.set(remaining);
                        }
                        press.input(SheetInput::Pressed(menu.default));
                    });
                    running.set(Some((menu.serial, task)));
                }
            }
        }
    });
    left()
}
