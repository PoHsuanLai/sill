//! The accounts sheet's root: while accountd has a sheet open and the banners' output is this one,
//! the surface maps itself and a window stands centred in it: quire's titlebar carrying the step's
//! title, over the step's own column. The consent question is quire's `ConsentBody` in the same window. Every part is quire's; what sill adds is the window's place
//! and look (`style.css`, tokens only) and where each event goes (`io.rs`).

use dioxus::prelude::*;
use ds::components::chrome::window_frame::WindowTitlebar;
use ds::prelude::*;
use ds_shell::accounts::model::{SessionOffer, StepTitle};
use ds_shell::prelude::{
    BrowserWait, ConsentBody, ProviderList, ReviewServices, ShowCode, SignInFailed, SignInForm,
    SignInWorking,
};
use shell_host::{OutputInfo, SurfaceHandle, Visibility};
use sill_model::accounts::sheet::{AccountsSheets, SheetAction};
use sill_services::accounts::sheet::AccountsSheetService;
use sill_ui_kit::hooks::{use_banner_output, use_service};
use sill_ui_kit::root::SurfaceRoot;

use super::ACCOUNTS_SHEET_CSS;
use super::continue_as::ContinueBody;
use super::events;
use super::io::{Io, use_io};
use super::props::{Purpose, Step};
use super::stage::{Drawn, Here, Mapped, drawn};

pub(super) fn accounts_sheet(output: OutputInfo) -> Element {
    let shown = use_drawn(output.clone());
    let current = shown();
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Sheet,
            extent: RootExtent::Viewport,
            AppStyle { css: ACCOUNTS_SHEET_CSS }
            if let Some(current) = current {
                // One card per step of one sheet: a new step mounts afresh and the keyboard
                // lands on its default control again.
                Card { key: "{current.handle.as_str()}-{slug(&current.step)}", drawn: current }
            }
        }
    }
}

/// The step's word, for its key and its `data-step`.
fn slug(step: &Step) -> &'static str {
    match step {
        Step::Consent(_) => "consent",
        Step::Providers(_) => "providers",
        Step::SignIn(_) => "sign-in",
        Step::Browser { .. } => "browser",
        Step::Code { .. } => "code",
        Step::Review(_) => "review",
        Step::Working { .. } => "working",
        Step::Failed { .. } => "failed",
    }
}

#[component]
fn Card(drawn: Drawn) -> Element {
    let io = use_io(drawn.handle.clone());
    let title = drawn.step.title();
    let step = slug(&drawn.step);
    rsx! {
        div { class: "sill-accounts-stage",
            div {
                class: "sill-accounts-window",
                role: "dialog",
                "aria-label": "{title}",
                "data-step": step,
                WindowTitlebar { title: title.clone(), lights: TrafficLights::Hidden }
                div { class: "sill-accounts-body", {body(drawn.step, io)} }
            }
        }
    }
}

/// One step's column, with every event sent to `io`.
fn body(step: Step, io: Io) -> Element {
    let act = move |action: Option<SheetAction>| {
        if let Some(action) = action {
            io.act.call(action);
        }
    };
    let cancel = move |()| io.act.call(SheetAction::Cancel);
    let back = move |()| io.act.call(SheetAction::Back);
    let session = match &step {
        Step::Consent(props) => props.session.clone(),
        _ => None,
    };
    match step {
        Step::Consent(props) if props.purpose == Purpose::ContinueAs => rsx! {
            ContinueBody {
                app: props.app,
                choices: props.choices,
                chosen: props.chosen,
                on_choose: move |key| act(events::chosen(&key)),
                on_answer: move |answer| act(events::consent(answer, session.as_ref())),
            }
        },
        Step::Consent(props) => rsx! {
            ConsentBody {
                session: match props.session {
                    Some(_) => SessionOffer::Offered,
                    None => SessionOffer::None,
                },
                app: props.app,
                request: props.request,
                choices: props.choices,
                chosen: props.chosen,
                on_choose: move |key| act(events::chosen(&key)),
                on_answer: move |answer| act(events::consent(answer, session.as_ref())),
            }
        },
        Step::Providers(props) => rsx! {
            ProviderList {
                providers: props.providers,
                query: props.query,
                cursor: props.cursor,
                on_query: move |query| io.act.call(SheetAction::Query(query)),
                on_cursor: move |pick| act(events::list_key(&pick).map(SheetAction::Cursor)),
                on_pick: move |pick| act(events::list_key(&pick).map(SheetAction::Pick)),
                on_cancel: cancel,
                title: StepTitle::Host,
            }
        },
        Step::SignIn(props) => rsx! {
            SignInForm {
                provider: props.provider,
                mark: props.mark,
                fields: props.fields,
                problem: props.problem,
                on_input: move |(role, text)| io.act.call(events::typed(role, text)),
                on_submit: move |()| io.act.call(SheetAction::Submit),
                on_back: back,
                on_cancel: cancel,
                title: StepTitle::Host,
            }
        },
        Step::Browser {
            provider,
            shown,
            url,
            copied,
        } => {
            rsx! {
                BrowserWait {
                    provider,
                    // The page prints what it is given and offers the same text to "Copy Link":
                    // it is given the short form, and the copy is of the whole address.
                    url: shown,
                    copied,
                    on_open_again: move |()| io.act.call(SheetAction::OpenAgain),
                    on_copy: move |_short: String| io.copy.call(url.clone()),
                    on_cancel: cancel,
                    title: StepTitle::Host,
                }
            }
        }
        Step::Code {
            provider,
            code,
            url,
            copied,
        } => rsx! {
            ShowCode {
                provider,
                code,
                url,
                copied,
                on_copy: move |text| io.copy.call(text),
                on_cancel: cancel,
                title: StepTitle::Host,
            }
        },
        Step::Review(props) => rsx! {
            ReviewServices {
                account: props.account,
                services: props.services,
                allow: props.allow,
                on_toggle: move |(key, check)| act(events::toggled(&key, check)),
                on_done: move |()| io.act.call(SheetAction::Confirm),
                on_back: back,
                on_cancel: cancel,
                title: StepTitle::Host,
            }
        },
        Step::Working {
            provider,
            waiting_for,
        } => rsx! {
            SignInWorking { provider, waiting_for, on_cancel: cancel, title: StepTitle::Host }
        },
        Step::Failed {
            provider,
            why,
            recovery,
            mark,
        } => rsx! {
            SignInFailed {
                provider,
                why,
                recovery,
                mark,
                on_retry: move |()| io.act.call(SheetAction::Retry),
                on_back: back,
                on_cancel: cancel,
                title: StepTitle::Host,
            }
        },
    }
}

/// The step on screen on `output`, following the service and the banners' output, and the
/// surface's mapping following it. Nothing to map without a host (a test, an SSR render).
fn use_drawn(output: OutputInfo) -> Memo<Option<Drawn>> {
    let sheets: ReadSignal<AccountsSheets> = use_service::<AccountsSheetService>();
    let banners = use_banner_output();
    let shown = use_memo(move || {
        let here = if banners.read().shows_on(output.id) {
            Here::Yes
        } else {
            Here::No
        };
        drawn(&sheets.read(), here)
    });
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Mapped::No));
    use_effect(move || {
        let mapped = Drawn::mapped(shown.read().as_ref());
        if *applied.peek() == mapped {
            return;
        }
        applied.set(mapped);
        if let Some(surface) = &surface {
            surface.set_visibility(match mapped {
                Mapped::Yes => Visibility::Shown,
                Mapped::No => Visibility::Hidden,
            });
        }
    });
    shown
}
