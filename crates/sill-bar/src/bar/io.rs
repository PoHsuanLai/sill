//! The bar's menu session: the tracking machine in a signal, its effects on the one popup, and
//! the context the bar's titles feed it through. The only effectful part of the menus; every
//! decision is `track::step`, `plan::effects` or `content::body`.

use ds::base::time::clock::sleep;
use ds::prelude::*;
use std::collections::HashMap;

use dioxus::prelude::*;
use shell_host::{HostRoot, OutputId, OutputInfo, PopupAnchor, PopupConfig, SurfaceHandle};
use sill_model::ids::OutputName;
use sill_settings::FocusMode;
use sill_ui_kit::links::FocusLink;
use sill_ui_kit::settings::SettingsFeed;
use tokio::sync::{mpsc, watch};

use super::content::{BarState, body, front_app};
use super::layer::layer;
use super::menu_popup::{BarPopup, PopupLink, SUBMENU_ROOM};
use super::menus::BarMenu;
use super::model::{BarInput, Handoff, PopupView, SessionDeps, status_box};
use super::pick::{BarPick, Command, commands};
use super::plan::{BarEffect, PopupReport, effects, popup_input};
use super::popup::menu_popup;
use super::popup::{BOTTOM, MAX_PX, TOP, placement};
use super::track::{Event, Target, Track, step};
use super::tray_input::{chroma_limit, row_image};
use crate::control_center::{PanelCap, height_cap};
use ds::base::time::stamp::FrameClock;
use sill_model::tray::TrayCommand;
use sill_services::{AppsService, CosmicWlService, TrayService};
use sill_ui_kit::hooks::{
    use_appearance, use_service, use_service_part, use_services, use_settings,
};
use sill_ui_kit::links::{CenterLink, CenterShown, ControlCenterLink, PowerMenuLink, Preselect};
use sill_ui_kit::menu::PopupLook;

/// How long a menu opened while the notification center is up waits for it to leave. On KWin
/// the center's unmapping moves the activation to the window under it, and a grabbing popup
/// opened before that move is dismissed by it at once; by this time the center
/// is gone and the move is over.
const CENTER_HANDOFF: std::time::Duration = std::time::Duration::from_millis(150);

/// The session as the bar's titles see it.
#[derive(Clone, Copy)]
pub(crate) struct MenuBar {
    feed: Callback<BarInput>,
    open: Signal<Option<BarMenu>>,
}

impl MenuBar {
    /// Feed the machine.
    pub(crate) fn feed(&self, input: BarInput) {
        self.feed.call(input);
    }

    /// Whether `menu` is the one showing (subscribes the caller).
    pub(crate) fn is_open(&self, menu: BarMenu) -> bool {
        (self.open)() == Some(menu)
    }

    /// Open `menu` as a click on its title would, or close it when it is the one showing:
    /// `sill control-center toggle`, which has no pointer event behind it.
    pub(crate) fn toggle(&self, menu: BarMenu) {
        let press = BarInput::Bar {
            input: Event::PressTitle(menu.id()),
            title: None,
        };
        let showing = *self.open.peek() == Some(menu);
        self.feed(press);
        if !showing {
            self.feed(BarInput::Bar {
                input: Event::Release(Target::Title(menu.id())),
                title: None,
            });
        }
    }
}

/// The popup channel of the open session: its view sender and the popup's report channel.
struct Link {
    view: Option<watch::Sender<Option<PopupView>>>,
    reports: mpsc::UnboundedSender<PopupReport>,
}

/// The mutable halves of a session, all `Copy` handles.
#[derive(Clone, Copy)]
struct Slots {
    track: Signal<Track>,
    open: Signal<Option<BarMenu>>,
    rects: Signal<HashMap<BarMenu, Rect>>,
    link: CopyValue<Link>,
    handoff: CopyValue<Handoff>,
    /// The session's own input, for the handoff's wake-up (set once the callback exists).
    feed: CopyValue<Option<Callback<BarInput>>>,
}

/// The bar on `output`: its menu session, provided to the titles as context, and the control
/// center asks it follows.
pub(super) fn use_bar(output: &OutputInfo, look: PopupLook) -> MenuBar {
    let file = use_settings()();
    let on = output.name.clone().map(OutputName);
    let state = BarState {
        app: front_app(
            &use_service_part::<CosmicWlService, _>(|wl| wl.toplevels.clone())(),
            &use_service::<AppsService>()(),
            on.as_ref(),
        ),
        control_center: file.control_center.clone(),
        tray: use_service::<TrayService>()(),
    };
    let focus = try_use_context::<FocusLink>().unwrap_or_else(|| FocusLink::fixed(FocusMode::Off));
    let bar = use_menu_session(SessionDeps {
        state,
        services: use_services(),
        surface: try_use_context::<SurfaceHandle>(),
        timing: super::track::timing(&file.menus),
        chroma: chroma_limit(use_appearance()().settings.icons.symbolic_chroma_max.0),
        look,
        namespace: layer(output, &file).namespace,
        output: output.id,
        power_menu: try_use_context::<PowerMenuLink>(),
        settings: use_context::<SettingsFeed>(),
        focus,
        output_name: on,
        bar_width: output.logical.width,
        cap: height_cap(output.logical.height, &file.bar, &file.control_center),
        status_box: status_box(output.logical.width, &file.bar),
        center: try_use_context::<CenterLink>(),
    });
    use_context_provider(|| bar);
    use_control_center_asks(bar, output.id);
    bar
}

/// Follow `sill control-center toggle` (the daemon's `ControlCenterLink`, absent in a demo or a
/// test): an ask for this bar's output toggles its control center.
fn use_control_center_asks(bar: MenuBar, output: OutputId) {
    let link = try_use_context::<ControlCenterLink>();
    use_hook(move || {
        let Some(link) = link else {
            return;
        };
        let mut asks = link.subscribe();
        // Only asks made after the bar subscribed.
        asks.borrow_and_update();
        spawn(async move {
            while asks.changed().await.is_ok() {
                let ask = *asks.borrow_and_update();
                if ask.output == Some(output) {
                    bar.toggle(BarMenu::ControlCenter);
                }
            }
        });
    });
}

/// Start (once) and update (every render) the bar's menu session.
pub(crate) fn use_menu_session(deps: SessionDeps) -> MenuBar {
    let clock =
        use_hook(|| try_consume_context::<FrameClock>().unwrap_or_else(FrameClock::started));
    let (reports_tx, reports_rx) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<PopupReport>();
        (tx, CopyValue::new(Some(rx)))
    });
    let slots = Slots {
        track: use_signal(Track::closed),
        open: use_signal(|| None),
        rects: use_signal(HashMap::new),
        link: use_hook(|| {
            CopyValue::new(Link {
                view: None,
                reports: reports_tx,
            })
        }),
        handoff: use_hook(|| CopyValue::new(Handoff::Idle)),
        feed: use_hook(|| CopyValue::new(None)),
    };
    refresh(slots, &deps);
    // The open tray item is gone (its app quit): its menu has nothing left to show.
    let vanished = matches!(
        *slots.open.peek(),
        Some(BarMenu::Tray(slot)) if deps.state.tray.items.get(usize::from(slot.0)).is_none()
    );
    let feed = use_callback(move |input: BarInput| handle(slots, &deps, clock, input));
    let mut own = slots.feed;
    if own.peek().is_none() {
        own.set(Some(feed));
    }
    use_effect(use_reactive!(|vanished| {
        if vanished {
            feed.call(BarInput::Bar {
                input: Event::OutsidePress,
                title: None,
            });
        }
    }));
    use_hook(move || {
        let mut rx = reports_rx;
        if let Some(mut reports) = rx.write().take() {
            spawn(async move {
                while let Some(report) = reports.recv().await {
                    feed.call(BarInput::Popup(report));
                }
            });
        }
    });
    MenuBar {
        feed,
        open: slots.open,
    }
}

/// Step the machine with `input` and carry out its effects.
fn handle(slots: Slots, deps: &SessionDeps, clock: FrameClock, input: BarInput) {
    let (input, picked) = match input {
        BarInput::Bar { input, title } => {
            let mut rects = slots.rects;
            if let Some((menu, rect)) = title
                && rects.peek().get(&menu) != Some(&rect)
            {
                rects.write().insert(menu, rect);
            }
            (input, None)
        }
        BarInput::Popup(report) => popup_input(report, slots.track.peek().open_menu().copied()),
        BarInput::CenterGone => {
            let mut handoff = slots.handoff;
            let waiting = handoff.replace(Handoff::Idle);
            if waiting == Handoff::Waiting
                && let Some(menu) = slots
                    .track
                    .peek()
                    .open_menu()
                    .copied()
                    .and_then(BarMenu::of)
            {
                show(slots, deps, menu);
            }
            return;
        }
    };
    let mut track = slots.track;
    let (next, outs) = step(track.peek().clone(), input, clock.now(), &deps.timing);
    if *track.peek() != next {
        track.set(next);
    }
    for effect in effects(outs, picked) {
        apply(slots, deps, effect);
    }
}

fn apply(slots: Slots, deps: &SessionDeps, effect: BarEffect) {
    let mut open = slots.open;
    let mut link = slots.link;
    match effect {
        BarEffect::Show(menu) => match center_open(deps) {
            // The center leaves first; the popup opens once it has.
            Some(center) => {
                center.close();
                let mut handoff = slots.handoff;
                handoff.set(Handoff::Waiting);
                open.set(Some(menu));
                if let Some(feed) = *slots.feed.peek() {
                    spawn(async move {
                        sleep(CENTER_HANDOFF).await;
                        feed.call(BarInput::CenterGone);
                    });
                }
            }
            None => show(slots, deps, menu),
        },
        BarEffect::Swap(menu) => {
            if let (Some(view), Some(tx)) = (view(slots, deps, menu), &link.peek().view) {
                tx.send_replace(Some(view));
                announce(deps, menu);
            }
            open.set(Some(menu));
        }
        BarEffect::Hide => {
            let mut handoff = slots.handoff;
            handoff.set(Handoff::Idle);
            if let Some(tx) = link.write().view.take() {
                tx.send_replace(None);
            }
            if open.peek().is_some() {
                open.set(None);
            }
        }
        BarEffect::Run(pick) => run(&pick, deps),
    }
}

/// The notification center, when it is open anywhere.
fn center_open(deps: &SessionDeps) -> Option<&CenterLink> {
    deps.center
        .as_ref()
        .filter(|center| center.now() != CenterShown::Closed)
}

/// Open the popup with `menu`'s view.
fn show(slots: Slots, deps: &SessionDeps, menu: BarMenu) {
    let mut open = slots.open;
    let mut link = slots.link;
    let Some(view) = view(slots, deps, menu) else {
        return;
    };
    let Some(surface) = &deps.surface else {
        return;
    };
    // Fitted to the document (`menu_popup`'s FitContent), which is the card and its room; the
    // cap is the tallest a panel goes, which no text menu (320 tall at most) nears.
    let config = PopupConfig {
        anchor: PopupAnchor::Rect,
        ..menu_popup(&deps.namespace, menu.side(), popup_cap(deps.cap))
    };
    let first = view.placement;
    let (tx, rx) = watch::channel(Some(view));
    let reports = link.peek().reports.clone();
    surface.open_popup(
        config,
        first,
        HostRoot::new(BarPopup)
            .context(PopupLink {
                view: rx,
                reports,
                timing: deps.timing,
                output: deps.output_name.clone(),
            })
            .context(deps.services.clone())
            .context(deps.settings.clone())
            .context(deps.focus.clone())
            .context(PanelCap(deps.cap)),
    );
    link.write().view = Some(tx);
    open.set(Some(menu));
    announce(deps, menu);
}

/// Tell a tray item its menu is about to show (dbusmenu `AboutToShow`), so the app can refresh
/// it; the other menus have nobody to tell.
fn announce(deps: &SessionDeps, menu: BarMenu) {
    if let BarMenu::Tray(slot) = menu
        && let Some(item) = deps.state.tray.items.get(usize::from(slot.0))
    {
        // The service may have stopped; then nothing is left to refresh.
        let _gone_is_dropped = deps.services.tray.control.send(TrayCommand::MenuOpening {
            item: item.id.clone(),
        });
    }
}

/// `menu`'s view now, placed under its title; `None` before the title has been measured. The
/// control center, which hangs from the bar's end rather than its item, is placed from the
/// status box's height when its item has not been measured (a toggle over IPC).
fn view(slots: Slots, deps: &SessionDeps, menu: BarMenu) -> Option<PopupView> {
    let measured = slots.rects.peek().get(&menu).copied();
    let rect = match (measured, menu) {
        (Some(rect), _) => rect,
        (None, BarMenu::ControlCenter) => deps.status_box,
        (None, _) => return None,
    };
    if let BarMenu::Tray(slot) = menu
        && deps.state.tray.items.get(usize::from(slot.0)).is_none()
    {
        return None;
    }
    let body = body(menu, &deps.state, &|icon| row_image(icon, deps.chroma));
    Some(PopupView {
        menu,
        placement: placement(rect, menu.side(), deps.bar_width),
        body,
        look: deps.look,
    })
}

/// The most the popup grows in either direction: the panel's cap and the room around it, and
/// never less than a text menu's maximum with a level of submenu beside it.
fn popup_cap(panel: i32) -> i32 {
    (panel + TOP + BOTTOM).max(MAX_PX + SUBMENU_ROOM)
}

/// Keep an open popup's contents current as the services change under it.
fn refresh(slots: Slots, deps: &SessionDeps) {
    let Some(menu) = *slots.open.peek() else {
        return;
    };
    let Some(next) = view(slots, deps, menu) else {
        return;
    };
    if let Some(tx) = &slots.link.peek().view {
        tx.send_if_modified(|shown| {
            let changed = shown.as_ref() != Some(&next);
            if changed {
                *shown = Some(next);
            }
            changed
        });
    }
}

/// Send `pick`'s commands. A service that has stopped drops them: the menu has closed and
/// there is nobody left to tell.
fn run(pick: &BarPick, deps: &SessionDeps) {
    let services = &deps.services;
    for command in commands(pick) {
        let _gone_is_dropped = match command {
            Command::Wl(c) => services.cosmic_wl.control.send(c),
            Command::Launch(c) => services.launch.control.send(c),
            Command::Tray(c) => services.tray.control.send(c),
            Command::PowerMenu => {
                if let Some(link) = &deps.power_menu {
                    link.open(deps.output, Preselect::Default);
                }
                Ok(())
            }
        };
    }
}
