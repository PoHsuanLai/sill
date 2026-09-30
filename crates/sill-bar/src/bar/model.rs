//! The bar's menu session as data: what its popup shows, what it is fed, what it needs from the
//! bar each render, and the notification-center handoff it waits on.

use ds::prelude::*;
use ds::style::tokens::shell_scale::SHELL_SCALE;
use shell_host::{OutputId, PopupPlacement, SurfaceHandle};
use sill_model::ids::OutputName;
use sill_settings::BarSettings;
use sill_ui_kit::links::FocusLink;
use sill_ui_kit::settings::SettingsFeed;

use super::content::{BarState, PopupBody};
use super::menus::BarMenu;
use super::plan::PopupReport;
use super::track::Event;
use ds::stack::menu_track::types::MenuTiming;
use sill_ui_kit::links::{CenterLink, PowerMenuLink};
use sill_ui_kit::menu::PopupLook;
use sill_ui_kit::services::Services;

/// Whether a bar menu waits for the notification center to leave before its popup opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Handoff {
    Idle,
    Waiting,
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
        input: Event,
        title: Option<(BarMenu, Rect)>,
    },
    /// What the popup reported.
    Popup(PopupReport),
    /// The notification center a menu closed has had time to leave ([`Handoff`]).
    CenterGone,
}

/// What the session needs from the bar each render.
pub(crate) struct SessionDeps {
    pub state: BarState,
    pub services: Services,
    pub surface: Option<SurfaceHandle>,
    pub timing: MenuTiming,
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
    pub status_box: Rect,
    /// The notification center, which a bar menu closes as it opens (one menu-bar panel at a
    /// time, as on macOS); absent in a demo or a test.
    pub center: Option<CenterLink>,
}

/// The last status item's slot, vertically centred in the bar, at its right end: the control
/// center's title rect when it opens with no pointer event behind it. The slot is quire's
/// (design/29): `max(--bar-status-w, box)` wide by the box tall.
pub(super) fn status_box(bar_width: i32, bar: &BarSettings) -> Rect {
    let side = f32::from(bar.status_icon_box_px.0);
    let width = side.max(f32::from(SHELL_SCALE.status_width.0));
    let height = f32::from(bar.height_px.0);
    Rect {
        origin: Point {
            x: Px(bar_width as f32 - width),
            y: Px((height - side) / 2.0),
        },
        size: Size {
            width: Px(width),
            height: Px(side),
        },
    }
}
