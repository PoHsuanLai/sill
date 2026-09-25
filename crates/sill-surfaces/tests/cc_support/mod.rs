//! The control center rendered to HTML on the fake services, for its render and lint tests:
//! the real `ControlCenter` in a quire `Ds` root, as the bar's popup draws it.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use dioxus::prelude::*;
use ds::{Appearance, Ds, Material};
use sill_services::{OutputName, Services};
use sill_settings::{FocusLink, FocusMode, SettingsFeed, ShellFile};
use sill_surfaces::surfaces::control_center::ControlCenter;

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

/// The control center on `services` and `file`, with Do Not Disturb on, as HTML.
pub fn render(services: Services, file: ShellFile) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let mut dom = VirtualDom::new(Panel);
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
