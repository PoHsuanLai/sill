//! The bar's menu session: the tracking machine in a signal, its effects on the one popup, and
//! the context the bar's titles feed it through. The only effectful part of the menus; every
//! decision is `menu_track::step`, `plan::effects` or `content::body`.

use std::collections::HashMap;

use dioxus::prelude::*;
use ds::{Alpha, Appearance, MaterialStack, ShellMetrics, SystemPrefs};
use shell_host::{
    OutputId, PopupAnchor, PopupConfig, PopupPlacement, PopupSize, Root, SurfaceHandle,
};
use sill_services::{OutputName, Services};
use sill_settings::{FocusLink, SettingsFeed};
use tokio::sync::{mpsc, watch};

use super::content::{BarState, PopupBody, body};
use super::geometry::{Around, MAX_PX, placement};
use super::menu_popup::{MenuPopup, PopupLink};
use super::menu_track::Target;
use super::menu_track::{MenuIn, MenuParams, MenuTrack, step};
use super::menus::BarMenu;
use super::pick::{BarPick, Command, commands};
use super::plan::{BarEffect, PopupReport, effects, popup_input};
use super::popup::menu_popup;
use crate::clock::FrameClock;
use crate::surfaces::control_center::fit::{Fit, device_rows, wifi_rows};
use crate::surfaces::control_center::{CcPage, PanelCap};
use crate::surfaces::power_menu::{PowerMenuLink, Preselect};

/// How a popup's `Ds` root resolves its scheme: the bar's own appearance, handed over so a
/// popup does not start a second settings watch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PopupLook {
    pub appearance: Appearance,
    pub system: SystemPrefs,
    pub tint: Alpha,
    /// The shell type scale, written around the popup's root (its menu's 22 px rows).
    pub metrics: ShellMetrics,
    /// The material stack its root hands the menu card.
    pub stack: MaterialStack,
}

/// What the popup shows: one menu's contents, where, and in which appearance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PopupView {
    pub menu: BarMenu,
    pub body: PopupBody,
    pub placement: PopupPlacement,
    pub look: PopupLook,
}

/// One input to the session.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BarInput {
    /// A pointer event on the bar, with the title's measured rect when it came from one.
    Bar {
        input: MenuIn,
        title: Option<(BarMenu, ds::Rect)>,
    },
    /// What the popup reported.
    Popup(PopupReport),
    /// The control center moved to `page` (the popup resizes to it).
    Page(CcPage),
}

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
            input: MenuIn::PressTitle(menu.id()),
            title: None,
        };
        let showing = *self.open.peek() == Some(menu);
        self.feed(press);
        if !showing {
            self.feed(BarInput::Bar {
                input: MenuIn::Release(Target::Title(menu.id())),
                title: None,
            });
        }
    }
}

/// What the session needs from the bar each render.
pub(crate) struct SessionDeps {
    pub state: BarState,
    pub services: Services,
    pub surface: Option<SurfaceHandle>,
    pub params: MenuParams,
    pub look: PopupLook,
    /// The bar's layer namespace, which names the popup's anchor in `menu_popup`.
    pub namespace: String,
    /// The bar's output, where the app menu's "Shut Down…" opens the power menu.
    pub output: OutputId,
    /// The power menu's shared state, when the daemon gave the bar one (a demo or a test may
    /// not).
    pub power_menu: Option<PowerMenuLink>,
    /// The popup root's contexts for the control center and the module dropdowns: the
    /// settings, the focus mode, the bar's output by name.
    pub settings: SettingsFeed,
    pub focus: FocusLink,
    pub output_name: Option<OutputName>,
    /// The bar's width (the control center hangs from its right end) and the panel's cap
    /// (`control_center::height_cap`).
    pub bar_width: i32,
    pub cap: i32,
    /// The status items' box, centred in the bar: where a menu opened with no pointer event
    /// (the control center's toggle) takes its title's rect from.
    pub status_box: ds::Rect,
}

/// The popup channel of the open session: its view sender and the popup's report channel.
struct Link {
    view: Option<watch::Sender<Option<PopupView>>>,
    reports: mpsc::UnboundedSender<PopupReport>,
    pages: mpsc::UnboundedSender<CcPage>,
}

/// The mutable halves of a session, all `Copy` handles.
#[derive(Clone, Copy)]
struct Slots {
    track: Signal<MenuTrack>,
    open: Signal<Option<BarMenu>>,
    rects: Signal<HashMap<BarMenu, ds::Rect>>,
    /// The control center's page, which sizes its popup.
    page: CopyValue<CcPage>,
    link: CopyValue<Link>,
}

/// Start (once) and update (every render) the bar's menu session.
pub(crate) fn use_menu_session(deps: SessionDeps) -> MenuBar {
    let clock = use_hook(|| {
        try_consume_context::<FrameClock>()
            .unwrap_or_else(|| FrameClock::new(std::time::Instant::now()))
    });
    let (reports_tx, reports_rx) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<PopupReport>();
        (tx, CopyValue::new(Some(rx)))
    });
    let (pages_tx, pages_rx) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<CcPage>();
        (tx, CopyValue::new(Some(rx)))
    });
    let slots = Slots {
        track: use_signal(|| MenuTrack::Closed),
        open: use_signal(|| None),
        rects: use_signal(HashMap::new),
        page: use_hook(|| CopyValue::new(CcPage::Modules)),
        link: use_hook(|| {
            CopyValue::new(Link {
                view: None,
                reports: reports_tx,
                pages: pages_tx,
            })
        }),
    };
    refresh(slots, &deps);
    let feed = use_callback(move |input: BarInput| handle(slots, &deps, clock, input));
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
    use_hook(move || {
        let mut rx = pages_rx;
        if let Some(mut pages) = rx.write().take() {
            spawn(async move {
                while let Some(page) = pages.recv().await {
                    feed.call(BarInput::Page(page));
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
        BarInput::Popup(report) => popup_input(report),
        BarInput::Page(page) => {
            let mut slot = slots.page;
            slot.set(page);
            // Read before the swap, which writes `open`.
            let showing = *slots.open.peek();
            if let Some(menu) = showing {
                apply(slots, deps, BarEffect::Swap(menu));
            }
            return;
        }
    };
    let mut track = slots.track;
    let (next, outs) = step(track.peek().clone(), input, clock.now(), &deps.params);
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
    let showing = *open.peek();
    if effect_menu(&effect).is_some_and(|menu| showing != Some(menu)) {
        // A menu shown afresh starts the control center on its grid.
        let mut page = slots.page;
        page.set(CcPage::Modules);
    }
    match effect {
        BarEffect::Show(menu) => {
            let Some(view) = view(slots, deps, menu) else {
                return;
            };
            let Some(surface) = &deps.surface else {
                return;
            };
            // Fixed, not `menu_popup`'s FitContent: the document holds only an absolutely
            // placed frame and an overlay, so shell-host's content fit measures it at nothing
            // and shrinks the popup to 1 x 1 (FINDINGS "Adoption of the bar gaps"). The size is
            // the geometry's estimate, room included.
            let config = PopupConfig {
                anchor: PopupAnchor::Rect,
                size: PopupSize::Fixed,
                ..menu_popup(&deps.namespace, menu.side(), MAX_PX)
            };
            let first = view.placement;
            let (tx, rx) = watch::channel(Some(view));
            let reports = link.peek().reports.clone();
            let pages = link.peek().pages.clone();
            surface.open_popup(
                config,
                first,
                Root::new(MenuPopup)
                    .context(PopupLink {
                        view: rx,
                        reports,
                        pages,
                        output: deps.output_name.clone(),
                    })
                    .context(deps.services.clone())
                    .context(deps.settings.clone())
                    .context(deps.focus.clone())
                    .context(PanelCap(deps.cap)),
            );
            link.write().view = Some(tx);
            open.set(Some(menu));
        }
        BarEffect::Swap(menu) => {
            if let (Some(view), Some(tx)) = (view(slots, deps, menu), &link.peek().view) {
                tx.send_replace(Some(view));
            }
            open.set(Some(menu));
        }
        BarEffect::Hide => {
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

/// The menu a show or a swap is for.
fn effect_menu(effect: &BarEffect) -> Option<BarMenu> {
    match effect {
        BarEffect::Show(menu) | BarEffect::Swap(menu) => Some(*menu),
        BarEffect::Hide | BarEffect::Run(_) => None,
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
    let body = body(menu, &deps.state);
    let around = Around {
        bar_width: deps.bar_width,
        control_center: &deps.state.control_center,
        fit: Fit {
            page: *slots.page.peek(),
            wifi_rows: wifi_rows(&deps.state.wifi),
            device_rows: device_rows(&deps.state.bluetooth),
            cap: deps.cap,
        },
    };
    Some(PopupView {
        menu,
        placement: placement(rect, menu.side(), &body, &around),
        body,
        look: deps.look,
    })
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
            Command::Wl(c) => services.wl.send(c),
            Command::Launch(c) => services.launch.send(c),
            Command::PowerMenu => {
                if let Some(link) = &deps.power_menu {
                    link.open(deps.output, Preselect::Default);
                }
                Ok(())
            }
        };
    }
}
