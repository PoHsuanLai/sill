//! The control center rendered to HTML on the fake services, for its render and lint tests:
//! the real `ControlCenter` in a quire `Ds` root, as the bar's popup draws it.

#![allow(dead_code)] // Each test binary uses its own subset.

pub mod details;
pub use sill_surfaces::harness::fake;
pub mod live;

use dioxus::prelude::*;
use ds::{Appearance, Ds, Material};
use sill_model::ids::OutputName;
use sill_settings::ControlCenterModule;
use sill_settings::{FocusLink, FocusMode, SettingsFeed, ShellFile};
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::control_center::{ControlCenter, ModuleDropdown};

/// The panel's root: a Popover `Ds`, the control center on the bar output `DP-1`.
#[component]
fn Panel() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover,
            ControlCenter {
                output: Some(OutputName("DP-1".to_owned())),
                on_close: move |()| {},
                on_page: move |_| {},
            }
        }
    }
}

/// A module's own bar dropdown (`control_center.menu_bar_*`), in the same root.
#[component]
fn Dropdown(module: ControlCenterModule) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover,
            ModuleDropdown {
                module,
                output: Some(OutputName("DP-1".to_owned())),
                on_close: move |()| {},
            }
        }
    }
}

/// The control center on `services` and `file`, with Do Not Disturb on, as HTML.
pub fn render(services: Services, file: ShellFile) -> String {
    render_dom(VirtualDom::new(Panel), services, file)
}

/// `module`'s bar dropdown on `services` and `file`, as HTML.
pub fn render_dropdown(services: Services, file: ShellFile, module: ControlCenterModule) -> String {
    let dom = VirtualDom::new_with_props(Dropdown, DropdownProps { module });
    render_dom(dom, services, file)
}

/// `dom` on `services` and `file`, with Do Not Disturb on, as HTML.
fn render_dom(mut dom: VirtualDom, services: Services, file: ShellFile) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(file));
    dom.provide_root_context(FocusLink::fixed(FocusMode::DoNotDisturb));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// How many times `needle` occurs in `html`.
pub fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}
