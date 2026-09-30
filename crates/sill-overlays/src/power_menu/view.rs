//! The power menu's render: one `Ds` root in `power_menu.material` (the active Space's look),
//! as large as the output (`RootExtent::Viewport`), holding quire's `Alert` centred on its `Sheet`
//! (it dims nothing): the question and what it does for the opening's action, then the actions
//! logind offers as the alert's buttons, the opening's action first (Return presses it), and
//! Cancel (Escape presses it). The alert is keyed by the opening, so each opening plays the
//! entrance and starts its focus over; it plays its own exit and says when it has settled
//! (`on_hidden`), where the menu closes and a chosen action runs.

use dioxus::prelude::*;
use ds::{Alert, AlertButton, AlertRole, Common, Icon, RootExtent, TextLine};
use shell_host::OutputInfo;
use sill_model::session::{Session, SessionAction};

use super::choice::answers;
use super::io::{MenuCtx, use_menu};
use super::labels::{action_label, consequence, prompt};
use super::layer::PANEL_ID;
use super::sheet::{MenuSession, SheetInput};
use sill_ui_kit::hooks::use_settings;
use sill_ui_kit::root::{ClickAway, SurfaceRoot};
use sill_ui_kit::shell_style::material;

/// The panel on `output`.
pub(super) fn power_menu(output: OutputInfo) -> Element {
    let file = use_settings()();
    let ctx = use_menu(output.id, file.power_menu.default_action);
    let menu = (ctx.menu)();
    let offers = (ctx.offers)();
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: material(file.power_menu.material),
            extent: RootExtent::Viewport,
            if let Some(menu) = menu {
                {alert(&ctx, menu, &offers)}
            }
        }
    }
}

/// The alert for one opening.
fn alert(ctx: &MenuCtx, menu: MenuSession, offers: &Session) -> Element {
    let (hidden, catch) = (ctx.clone(), ctx.clone());
    let buttons = buttons(ctx, menu, offers);
    rsx! {
        Fragment { key: "{menu.serial.0}",
            // A sheet dims nothing and catches no click (macOS; design/30): a press outside it is
            // this surface's own, so it closes the menu as the scrim did.
            ClickAway { onpress: move |()| catch.input(SheetInput::Dismissed) }
            Alert {
                title: prompt(menu.default),
                message: Some(TextLine::from(consequence(menu.default))),
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

/// The alert's answers: each offered action, the opening's first, and Cancel.
fn buttons(ctx: &MenuCtx, menu: MenuSession, offers: &Session) -> Vec<AlertButton> {
    let actions = answers(menu.default, offers)
        .into_iter()
        .enumerate()
        .map(|(place, action)| {
            let press = ctx.clone();
            AlertButton::new(
                action_label(action),
                role(action, place),
                EventHandler::new(move |()| press.input(SheetInput::Pressed(action))),
            )
        });
    let cancel = ctx.clone();
    let cancel = AlertButton::new(
        "Cancel",
        AlertRole::Cancel,
        EventHandler::new(move |()| cancel.input(SheetInput::Dismissed)),
    );
    actions.chain(std::iter::once(cancel)).collect()
}

/// What an action's button means. Restart and Shut Down end every running app, so they are
/// destructive, drawn in red; but the alert never makes a destructive button its default, so the
/// first button, the one Return presses, is a plain one whichever action it is.
fn role(action: SessionAction, place: usize) -> AlertRole {
    match action {
        SessionAction::Restart | SessionAction::PowerOff if place > 0 => AlertRole::Destructive,
        SessionAction::Restart
        | SessionAction::PowerOff
        | SessionAction::Lock
        | SessionAction::LogOut
        | SessionAction::Suspend => AlertRole::Normal,
    }
}
