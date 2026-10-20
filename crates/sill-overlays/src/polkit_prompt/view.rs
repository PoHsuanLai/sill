//! The polkit prompt's root.
//!
//! While `use_service::<PolkitAgentService>().current` is a request and the `BannerLink` names this
//! output, the surface maps itself (`SurfaceHandle::set_visibility`), and a `Ds { material: Sheet,
//! extent: Viewport }` holds `sill_shell_kit::prelude::PolkitPrompt` from [`super::model::prompt_view`] (keyed by the cookie,
//! so a new request enters again), `caps` from the last key event's modifiers, `onsubmit` sending
//! `PolkitCommand::Answer { cookie, secret: Secret::new(text) }` (the text moves into the `Secret`
//! at once and is never kept in a signal), `oncancel` sending `PolkitCommand::Dismiss(cookie)`;
//! when the request goes away `shown` turns `Hidden` and the surface unmaps at `on_hidden`
//! (`sill_ui_kit::sheet_stage`). Nothing is styled under `.ds-polkit*`; the only markup of sill's own is the
//! wrapper that hears the keys' modifiers, since quire's sheet floats in the root's overlay host.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::auth::Secret;
use sill_model::polkit_agent::{Cookie, PolkitCommand};
use sill_services::{Control, PolkitAgentService};
use sill_shell_kit::lock::vocab::CapsLock;
use sill_shell_kit::prelude::*;

use super::surface::PANEL_ID;
use super::model::{PolkitPromptView, prompt_view};
use sill_ui_kit::hooks::{use_banner_output, use_control, use_service};
use sill_ui_kit::root::SurfaceRoot;
use sill_ui_kit::sheet_stage::{SheetStage, settle, use_sheet_stage};

pub(super) fn polkit_prompt(output: OutputInfo) -> Element {
    let stage = use_stage(output.clone());
    let control = use_control::<PolkitAgentService>();
    let mut caps = use_signal(|| CapsLock::Off);
    let drawn = stage().drawn().map(|(view, shown)| (view.clone(), shown));
    rsx! {
        div {
            onkeydown: move |event: KeyboardEvent| caps.set(caps_of(event.modifiers())),
            SurfaceRoot {
                output,
                material: Material::Sheet,
                extent: RootExtent::Viewport,
                if let Some((view, shown)) = drawn {
                    {prompt(view, shown, caps(), stage, control)}
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
    shown: Shown,
    caps: CapsLock,
    stage: Signal<SheetStage<PolkitPromptView>>,
    control: Control<PolkitCommand>,
) -> Element {
    let (submit, cancel) = (control.clone(), control);
    let (answer_to, dismiss) = (view.cookie.clone(), view.cookie.clone());
    let key = view.cookie.0.clone();
    rsx! {
        PolkitPrompt {
            key: "{key}",
            action: TextLine::from(view.action),
            detail: view.detail.map(TextLine::from),
            user: view.user,
            state: view.state,
            caps,
            // The sheet carries the blur region's id, so the layer's
            // `RegionSpec::Element(PANEL_ID)` resolves.
            common: Common { id: Some(PANEL_ID.to_owned()), ..Common::default() },
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

/// The stage on `output`, following the agent and the banners' output.
fn use_stage(output: OutputInfo) -> Signal<SheetStage<PolkitPromptView>> {
    let polkit = use_service::<PolkitAgentService>();
    let shows = use_banner_output();
    use_sheet_stage(move || {
        shows
            .read()
            .shows_on(output.id)
            .then(|| prompt_view(&polkit.read()))
            .flatten()
    })
}
