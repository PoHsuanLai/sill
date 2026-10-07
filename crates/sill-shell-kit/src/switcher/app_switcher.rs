//! AppSwitcher: the Cmd+Tab panel (design/13-BEHAVIOUR-menus-windows.md section 13.3.5;
//! design/20-SURFACES.md section 1.11; design/04-COMPONENTS.md section 43). One row of app
//! icons on the Osd material with the Space gradient, as the OSD card draws it; the selection a rounded square of `--f-pill` that springs from
//! cell to cell, the selected app's name under it. The component only draws and reports: the
//! show delay, the keys, the modifier's release and the MRU order are the shell's.

use crate::kept::use_kept;
use crate::switcher::switcher_fit::{SwitcherMetrics, fit};
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::base::vocab::{Selection, Shown};
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::overlays::tooltip::{Hint, HintSide};
use ds::motion::detail::touch::Touch;
use ds::motion::hover_intent::HoverProfile;
use ds::motion::presence::{Exit, Presence};
use ds::motion::roster::{RowPitch, presence_slug};
use ds::motion::use_roster::{LeaveBy, RosterSpec, use_roster};
use ds::motion::{
    spring_spec::{SpringResponse, SpringSpec},
    timeline::spring::PxPerUnit,
    use_spring::use_spring,
};
use ds::root::common::Common;
use ds::style::icon::family::PlateFamily;
use ds::style::icon::render::{IconPx, IconSize};

/// An application in the switcher, by the shell's own id for it (its app id).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppKey(pub String);

/// One application's tile.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitcherApp {
    /// Which application: what `onhover` and `onactivate` report.
    pub key: AppKey,
    /// Its name, shown under the tile while it is selected.
    pub name: String,
    /// Its icon: a glyph, a symbolic icon or its picture. The switcher sizes a glyph itself; an
    /// external icon is stretched to the cell's icon square.
    pub icon: IconSource,
    /// The app plate a glyph sits on, as the dock draws a placeholder (design/08 section 2);
    /// `None` for an app whose own picture is its plate.
    pub plate: Option<PlateFamily>,
}

impl SwitcherApp {
    /// A tile with no plate.
    pub fn new(key: impl Into<String>, name: impl Into<String>, icon: IconSource) -> Self {
        SwitcherApp {
            key: AppKey(key.into()),
            name: name.into(),
            icon,
            plate: None,
        }
    }
}

/// The switcher panel. `apps` in the shell's order (most recent first), `selected` the one the
/// selection is on. `output` is the width of the output it is shown on: past `output - 64` the
/// icons shrink to `metrics.min_icon` and then the row scrolls so the selection stays in view
/// (`None`: never shrunk). The pointer resting on a tile calls `onhover` with its key (the
/// shell selects it); a click calls `onactivate`. An app the shell stops listing (quit) leaves
/// as a roster row does, fading and sliding up over `--t-quick`, and stays drawn until that has
/// settled. Place it in a transparent Osd root
/// (`Ds { material: Material::Osd, chrome: Some(RootChrome::Transparent), .. }`); it appears
/// with `fade` at `--t-quick`, with no pop.
#[component]
pub fn AppSwitcher(
    apps: Vec<SwitcherApp>,
    selected: AppKey,
    #[props(default)] output: Option<Px>,
    #[props(default)] metrics: SwitcherMetrics,
    onhover: EventHandler<AppKey>,
    onactivate: EventHandler<AppKey>,
    #[props(default)] common: Common,
) -> Element {
    let kept = use_kept(
        apps.iter()
            .map(|app| (app.key.clone(), app.clone()))
            .collect(),
    );
    let roster = use_roster(
        apps.iter().map(|app| app.key.clone()).collect::<Vec<_>>(),
        RosterSpec {
            leave: LeaveBy::Delist,
            exit: Exit::Row,
            pitch: RowPitch(Px(metrics.cell.0 + metrics.gap.0)),
            on_settled: Some(EventHandler::new(move |key: AppKey| kept.forget(&key))),
        },
    );
    let drawn: Vec<(SwitcherApp, Presence)> = roster
        .entries()
        .into_iter()
        .filter_map(|entry| Some((kept.of(&entry.key)?, entry.presence)))
        .collect();
    let at = drawn
        .iter()
        .position(|(app, _)| app.key == selected)
        .unwrap_or_default();
    let row = fit(drawn.len(), at, metrics, output);
    // Driven motion (design/05 section 14, H1): the selection ring springs from cell to cell,
    // redirecting from where it is when the selection moves on mid-slide.
    let pitch = PxPerUnit(row.cell.0 + metrics.gap.0);
    let spec = SpringSpec::for_touch(Touch::Remote).response(SpringResponse::Quick);
    let ring = use_spring(at as f32, spec, pitch);
    let style = format!(
        "--switcher-cell:{}px;--switcher-icon:{}px;--switcher-gap:{}px;--switcher-view:{}px;--switcher-shift:{}px;--switcher-at:{}",
        row.cell.0,
        row.icon.0,
        metrics.gap.0,
        row.view.0,
        row.shift.0,
        ring.css()
    );
    let size = IconSize::Px(IconPx(row.icon.0.clamp(0.0, 255.0) as u8));
    let class = common.class("ds-switcher");
    let data = common.data_attributes();
    let label = common
        .aria_label
        .clone()
        .unwrap_or_else(|| "Applications".to_owned());
    rsx! {
        div {
            class,
            id: common.id.clone(),
            role: "listbox",
            "aria-label": "{label}",
            style,
            onmounted: move |event| common.mounted(event),
            ..data,
            div { class: "ds-frame",
                div { class: "ds-grain" }
            }
            div { class: "ds-switcher-view",
                div { class: "ds-switcher-row",
                    span { class: "ds-switcher-ring", "aria-hidden": "true" }
                    for (app , presence) in drawn {
                        {tile(app.clone(), presence, Selection::of(&app.key, &selected), size, onhover, onactivate)}
                    }
                }
            }
        }
    }
}

/// One tile: its icon, and its name as a label above it, up while it is selected.
fn tile(
    app: SwitcherApp,
    presence: Presence,
    selection: Selection,
    size: IconSize,
    onhover: EventHandler<AppKey>,
    onactivate: EventHandler<AppKey>,
) -> Element {
    let shown = match selection {
        Selection::Selected => Shown::Visible,
        Selection::Unselected => Shown::Hidden,
    };
    let hovered = app.key.clone();
    let activated = app.key.clone();
    let name = app.name.clone();
    rsx! {
        div {
            key: "{app.key.0}",
            class: "ds-switcher-cell",
            role: "option",
            "aria-selected": selection.aria(),
            "aria-label": "{name}",
            "data-selected": selection.slug(),
            "data-presence": presence_slug(presence, None),
            onpointerenter: move |_| onhover.call(hovered.clone()),
            onclick: move |_| onactivate.call(activated.clone()),
            Hint { text: app.name, shown, profile: HoverProfile::Label, side: HintSide::Above, root: "ds-tooltip",
                span { class: "ds-switcher-icon",
                    IconView { source: app.icon, size, plate: app.plate }
                }
            }
        }
    }
}
