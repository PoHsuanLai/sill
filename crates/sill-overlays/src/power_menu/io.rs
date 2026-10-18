//! The effects end of the menu: follow the [`PowerMenuLink`] onto the surface's visibility and
//! an opening's [`MenuSession`], feed [`advance`] the alert's answers and dismissals, and hand a
//! chosen action to the session service once quire's `Sheet` has played its exit (its
//! `on_hidden`). Once hidden, nothing runs and nothing asks for a frame. A daemon whose session
//! actions are a dry run (`sill run --session-dry-run`) says so in the `power_menu.backend=` line,
//! which the acceptance script proves before it presses Enter.

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, Visibility, use_watch};
use sill_model::session::{Session, SessionAction, SessionCommand};
use sill_services::Control;
use sill_services::env::ActionReach;
use sill_settings::PowerMenuDefault;
use sill_ui_kit::services::Services;

use super::choice::{initial, offered};
use super::follow::{Follow, follow};
use super::labels::action_label;
use super::sheet::{MenuSession, SheetInput, Then, advance};
use sill_ui_kit::links::{OpenSerial, PanelFocus, Preselect};
use sill_ui_kit::links::{PowerMenuLink, PowerMenuOpening, Reveal};
use sill_ui_kit::sheet_stage::{ExitOut, SheetExit};

/// Everything the menu's handlers reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct MenuCtx {
    link: PowerMenuLink,
    control: Control<SessionCommand>,
    pub offers: ReadSignal<Session>,
    pub menu: Signal<Option<MenuSession>>,
}

/// The menu on `output`, opening focused per `default`.
pub(super) fn use_menu(output: OutputId, default: PowerMenuDefault) -> MenuCtx {
    let link = use_context::<PowerMenuLink>();
    let services = use_context::<Services>();
    let control = use_hook(|| {
        eprintln!(
            "power_menu.backend={} output={}",
            reach_word(services.session_actions.reach()),
            output.0
        );
        services.session.control.clone()
    });
    let ctx = MenuCtx {
        offers: use_watch(services.session.state.clone()),
        link: link.clone(),
        control,
        menu: use_signal(|| None),
    };
    let shown = use_watch(link.subscribe());
    use_follow_visibility(shown, output);
    use_openings(ctx.clone(), shown, output, default);
    use_offers_arriving(ctx.clone(), default);
    ctx
}

/// The log word for `reach`: `live` when logind carries the actions out, `dry-run` when they are
/// only recorded.
fn reach_word(reach: ActionReach) -> &'static str {
    match reach {
        ActionReach::Logind => "live",
        ActionReach::DryRun => "dry-run",
    }
}

/// Start an opening's session on every opening on this output, show a leaving sheet again when
/// it is opened again, and drop the session once hidden.
fn use_openings(
    ctx: MenuCtx,
    shown: ReadSignal<Reveal<PowerMenuOpening>>,
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
                let default = initial(opening.preselect, *wanted.peek(), &ctx.offers.peek());
                menu.set(Some(MenuSession::opened(opening.serial, default)));
                let _gone_is_inert = ctx.control.send(SessionCommand::Refresh);
                eprintln!("power_menu.opened default={}", action_label(default));
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

/// logind answers after the menu has opened (`refresh` is asynchronous, and the first answer on a
/// fresh session can come late): when the offers change under an open menu, say so and, if the
/// action it is about is not offered, move it to one that is, so the buttons never wait on a
/// reopen.
fn use_offers_arriving(ctx: MenuCtx, default: PowerMenuDefault) {
    let mut menu = ctx.menu;
    use_effect(move || {
        let offers = *ctx.offers.read();
        let Some(now) = *menu.peek() else {
            return;
        };
        eprintln!("power_menu.offers={}", offered_labels(&offers));
        let about = initial(Preselect::Action(now.default), default, &offers);
        if about != now.default && now.phase == SheetExit::Open {
            menu.set(Some(MenuSession {
                default: about,
                ..now
            }));
        }
    });
}

/// Print `power_menu.keyboard=held` once per opening, when the surface first has the keyboard
/// (the daemon feeds the link its focus): `dev/accept/power-menu.sh --real-input` sends real keys only then.
fn use_keyboard_line(shown: ReadSignal<Reveal<PowerMenuOpening>>, output: OutputId) {
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

/// Map this surface while the menu shows on `output`, unmap it otherwise. Nothing to do without
/// a host (a test, an SSR render).
fn use_follow_visibility(shown: ReadSignal<Reveal<PowerMenuOpening>>, output: OutputId) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visibility>));
    use_effect(move || {
        let visible = shown.read().shows_on(output);
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.set_visibility(visible);
        }
    });
}

/// The actions `session` offers, in the buttons' order, as their labels joined by commas (the
/// log line `dev/accept/power-menu.sh` reads the arrows' path from).
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
        match effect {
            ExitOut::Nothing => {}
            ExitOut::Left => self.link.leaving(),
            ExitOut::Finished(then) => self.finish(then),
        }
    }

    /// Close, then carry out a chosen action.
    fn finish(&self, then: Then) {
        self.link.close();
        if let Then::Run(action) = then
            && let Err(error) = self.control.send(SessionCommand::Run(action))
        {
            eprintln!("sill: power menu: {}: {error}", action_label(action));
        }
    }
}
