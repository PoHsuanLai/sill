//! The polkit prompt's root.
//!
//! While `use_polkit().current` is a request and the `BannerLink` names this output, the
//! surface maps itself (`SurfaceOp::SetVisibility`), and a `Ds { material: Sheet, extent:
//! Viewport }` holds `ds::PolkitPrompt` from [`super::prompt_view`] (keyed by the cookie, so a
//! new request enters again), `caps` from the last key event's modifiers, `onsubmit` sending
//! `PolkitCommand::Answer { cookie, secret: Secret::new(text) }` (the text moves into the
//! `Secret` at once and is never kept in a signal), `oncancel` sending
//! `PolkitCommand::Dismiss(cookie)`; when the request goes away `shown` turns `Hidden` and the
//! surface unmaps at `on_hidden` ([`super::stage`]). Nothing is styled under `.ds-polkit*`
//! (quire CONSUMING "Lock, polkit and switcher"); the only markup of sill's own is the wrapper
//! that hears the keys' modifiers, since quire's sheet floats in the root's overlay host.

use dioxus::prelude::*;
use ds::{BlurState, CapsLock, Ds, Material, PolkitPrompt, RootExtent, Text};
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_services::auth::Secret;
use sill_services::polkit_agent::{Cookie, PolkitCommand};
use sill_services::{Control, OutputName};

use super::stage::{Mapped, Stage, StageIn, advance};
use super::view::{PolkitPromptView, prompt_view};
use crate::hooks::{
    use_appearance, use_ds_scale, use_polkit, use_services, use_settings, use_workspaces,
};
use crate::surfaces::banners::BannerLink;
use crate::surfaces::launcher::panel::active_look;

pub(super) fn polkit_prompt(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(&use_workspaces()(), on.as_ref(), &file.spaces);
    let stage = use_stage(output);
    let control = use_services().polkit_control;
    let mut caps = use_signal(|| CapsLock::Off);
    let drawn = stage().drawn().map(|(view, shown)| (view.clone(), shown));
    rsx! {
        div {
            onkeydown: move |event: KeyboardEvent| caps.set(caps_of(event.modifiers())),
            Ds {
                scale: ds_scale,
                appearance: environment.settings.appearance.appearance(),
                system: environment.system,
                look,
                material: Material::Sheet,
                extent: RootExtent::Viewport,
                blur: BlurState::Available,
                tint_alpha: Some(environment.tint_alpha()),
                stack: Some(environment.material_stack()),
                if let Some((view, shown)) = drawn {
                    {prompt(view, shown, caps(), stage, control.clone())}
                }
            }
        }
    }
}

/// Caps Lock from a key event's modifiers (xkb's locked state, through shell-host).
fn caps_of(modifiers: Modifiers) -> CapsLock {
    if modifiers.contains(Modifiers::CAPS_LOCK) {
        CapsLock::On
    } else {
        CapsLock::Off
    }
}

/// quire's prompt for `view`.
fn prompt(
    view: PolkitPromptView,
    shown: ds::Shown,
    caps: CapsLock,
    stage: Signal<Stage>,
    control: Control<PolkitCommand>,
) -> Element {
    let (submit, cancel) = (control.clone(), control);
    let (answer_to, dismiss) = (view.cookie.clone(), view.cookie.clone());
    let key = view.cookie.0.clone();
    rsx! {
        PolkitPrompt {
            key: "{key}",
            action: Text::from(view.action),
            detail: view.detail.map(Text::from),
            user: view.user,
            state: view.state,
            caps,
            // The sheet carries the blur region's id (quire Q260), so `layer.rs`'s
            // `RegionSpec::Element(PANEL_ID)` resolves.
            panel_id: Some(super::PANEL_ID.to_owned()),
            oninput: move |_: String| {},
            onsubmit: move |text: String| {
                let secret = Secret::new(text);
                let cookie: Cookie = answer_to.clone();
                // A stopped agent has no request to answer; the prompt goes with it.
                let _ = submit.send(PolkitCommand::Answer { cookie, secret });
            },
            oncancel: move |()| {
                let _ = cancel.send(PolkitCommand::Dismiss(dismiss.clone()));
            },
            shown: Some(shown),
            on_hidden: move |()| settle(stage),
        }
    }
}

fn settle(mut stage: Signal<Stage>) {
    let next = advance(stage.peek().clone(), StageIn::Settled);
    if *stage.peek() != next {
        stage.set(next);
    }
}

/// The stage on `output`, following the agent and the banners' output, and the surface's
/// mapping following the stage. Nothing to map without a host (a test, an SSR render).
fn use_stage(output: OutputInfo) -> Signal<Stage> {
    let polkit = use_polkit();
    let link = use_hook(|| try_consume_context::<BannerLink>().unwrap_or_default());
    let shows = use_watch(link.subscribe());
    let mut stage = use_signal(Stage::default);
    use_effect(move || {
        let wanted = shows
            .read()
            .shows_on(output.id)
            .then(|| prompt_view(&polkit.read()))
            .flatten();
        let next = advance(stage.peek().clone(), StageIn::Want(wanted));
        if *stage.peek() != next {
            stage.set(next);
        }
    });
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Mapped::No));
    use_effect(move || {
        let mapped = stage.read().mapped();
        if *applied.peek() == mapped {
            return;
        }
        applied.set(mapped);
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match mapped {
                Mapped::Yes => Visibility::Shown,
                Mapped::No => Visibility::Hidden,
            }));
        }
    });
    stage
}
