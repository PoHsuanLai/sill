//! The effects end of the menu: follow the [`PowerMenuLink`] onto the surface's visibility and
//! an opening's [`MenuSession`], feed [`advance`] the keys, clicks and dismissals, move the
//! keyboard focus onto the focused button, and hand a chosen action to the [`PowerMenuBackend`]
//! once quire's `Sheet` has played its exit (its `on_hidden`). Once hidden, nothing runs and
//! nothing asks for a frame.

use std::rc::Rc;

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_services::{Services, Session, SessionAction};
use sill_settings::PowerMenuDefault;

use crate::surfaces::launcher::presence::{OpenSerial, PanelFocus, Visible};
use crate::surfaces::power_menu::backend::{DryRunSession, PowerMenuBackend};
use crate::surfaces::power_menu::follow::{Follow, follow};
use crate::surfaces::power_menu::keymap::power_menu_key;
use crate::surfaces::power_menu::keys::{initial, offered};
use crate::surfaces::power_menu::labels::action_label;
use crate::surfaces::power_menu::presence::{PowerMenuLink, PowerMenuShown};
use crate::surfaces::power_menu::sheet::{MenuSession, SheetEffect, SheetInput, Then, advance};

/// The buttons' elements, for moving the keyboard focus.
type Buttons = Vec<(SessionAction, Rc<MountedData>)>;

/// Everything the menu's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(crate) struct MenuCtx {
    link: PowerMenuLink,
    backend: PowerMenuBackend,
    pub offers: ReadSignal<Session>,
    pub menu: Signal<Option<MenuSession>>,
    buttons: Signal<Buttons>,
}

/// The menu on `output`, opening focused per `default`.
pub(crate) fn use_menu(output: OutputId, default: PowerMenuDefault) -> MenuCtx {
    let link = use_context::<PowerMenuLink>();
    let services = try_use_context::<Services>();
    let backend = use_hook(|| {
        let backend = try_consume_context::<PowerMenuBackend>()
            .or_else(|| services.as_ref().map(PowerMenuBackend::live))
            .unwrap_or_else(|| {
                PowerMenuBackend::dry_run(DryRunSession::new(Session::default(), None))
            });
        eprintln!(
            "power_menu.backend={} output={}",
            backend.0.kind().name(),
            output.0
        );
        backend
    });
    let ctx = MenuCtx {
        offers: use_watch(backend.0.offers()),
        link: link.clone(),
        backend,
        menu: use_signal(|| None),
        buttons: use_signal(Vec::new),
    };
    let shown = use_watch(link.subscribe());
    use_follow_visibility(shown, output);
    use_openings(ctx.clone(), shown, output, default);
    use_button_focus(ctx.clone());
    crate::surfaces::power_menu::script::use_script(ctx.clone());
    ctx
}

/// Start an opening's session on every opening on this output, show a leaving sheet again when
/// it is opened again, and drop the session once hidden.
fn use_openings(
    ctx: MenuCtx,
    shown: ReadSignal<PowerMenuShown>,
    output: OutputId,
    default: PowerMenuDefault,
) {
    let mut wanted = use_hook(|| CopyValue::new(default));
    if *wanted.peek() != default {
        wanted.set(default);
    }
    let mut menu = ctx.menu;
    use_effect(move || {
        let opening = shown
            .read()
            .opening()
            .filter(|o| o.output == output)
            .cloned();
        let now = *menu.peek();
        match (follow(opening.as_ref(), now), opening) {
            (Follow::Start, Some(opening)) => {
                let keys = initial(opening.preselect, *wanted.peek(), &ctx.offers.peek());
                menu.set(Some(MenuSession::opened(opening.serial, keys)));
                ctx.backend.0.refresh();
                eprintln!("power_menu.opened focus={}", action_label(keys.focused));
                eprintln!("power_menu.offers={}", offered_labels(&ctx.offers.peek()));
            }
            (Follow::Reshow, _) => {
                ctx.input(SheetInput::Reshown);
                eprintln!("power_menu.reshown");
            }
            (Follow::Drop, _) => menu.set(None),
            _ => {}
        }
    });
    use_keyboard_line(shown, output);
}

/// Print `power_menu.keyboard=held` once per opening, when the surface first has the keyboard
/// (the daemon feeds the link its focus): `dev/accept-power-menu.sh` injects keys only then.
fn use_keyboard_line(shown: ReadSignal<PowerMenuShown>, output: OutputId) {
    let mut seen = use_hook(|| CopyValue::new(None::<(OpenSerial, PanelFocus)>));
    use_effect(move || {
        let now = shown
            .read()
            .opening()
            .filter(|o| o.output == output)
            .map(|o| (o.serial, o.focus));
        if now == *seen.peek() {
            return;
        }
        seen.set(now);
        if let Some((_, PanelFocus::Held)) = now {
            eprintln!("power_menu.keyboard=held");
        }
    });
}

/// Give the focused button the keyboard, so quire's focus ring moves with the arrows.
fn use_button_focus(ctx: MenuCtx) {
    let (menu, buttons) = (ctx.menu, ctx.buttons);
    use_effect(move || {
        let focused = menu().map(|m| m.keys.focused);
        let element = focused.and_then(|focused| {
            buttons
                .read()
                .iter()
                .find(|(action, _)| *action == focused)
                .map(|(_, element)| element.clone())
        });
        if let Some(element) = element {
            ds::focus_soon(element);
        }
    });
}

/// Map this surface while the menu shows on `output`, unmap it otherwise. Nothing to do without
/// a host (a test, an SSR render).
fn use_follow_visibility(shown: ReadSignal<PowerMenuShown>, output: OutputId) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visible>));
    use_effect(move || {
        let visible = shown.read().shows_on(output);
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match visible {
                Visible::Yes => Visibility::Shown,
                Visible::No => Visibility::Hidden,
            }));
        }
    });
}

/// The actions `session` offers, in the buttons' order, as their labels joined by commas (the
/// log line `dev/accept-power-menu.sh` reads the arrows' path from).
fn offered_labels(session: &Session) -> String {
    SessionAction::POWER_MENU
        .into_iter()
        .filter(|action| offered(session, *action))
        .map(action_label)
        .collect::<Vec<_>>()
        .join(",")
}

impl MenuCtx {
    /// `input` through the sheet's machine, then its effect.
    pub fn input(&self, input: SheetInput) {
        let Some(current) = *self.menu.peek() else {
            return;
        };
        let (next, effect) = advance(current, input, &self.offers.peek());
        if next != current {
            let mut menu = self.menu;
            menu.set(Some(next));
        }
        if next.keys != current.keys {
            eprintln!("power_menu.focus={}", action_label(next.keys.focused));
        }
        match effect {
            SheetEffect::Nothing => {}
            SheetEffect::Leave => self.link.leaving(),
            SheetEffect::Finish(then) => self.finish(then),
        }
    }

    /// A key reached the surface: the menu's own keys are taken from Blitz (Tab would move the
    /// focus, Enter would also click the focused button).
    pub fn key(&self, event: &KeyboardEvent) {
        if let Some(key) = power_menu_key(&event.key(), event.modifiers()) {
            event.prevent_default();
            event.stop_propagation();
            self.input(SheetInput::Key(key));
        }
    }

    /// A button's element, once mounted.
    pub fn mounted(&self, action: SessionAction, element: Rc<MountedData>) {
        let mut buttons = self.buttons;
        buttons.with_mut(|buttons| {
            buttons.retain(|(a, _)| *a != action);
            buttons.push((action, element));
        });
    }

    /// Close, then carry out a chosen action.
    fn finish(&self, then: Then) {
        self.link.close();
        if let Then::Run(action) = then
            && let Err(error) = self.backend.0.run(action)
        {
            eprintln!("sill: power menu: {}: {error}", action_label(action));
        }
    }
}
