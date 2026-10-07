//! A bar item hands its pointer's press, release and entry to its owner itself, with no wrapper
//! element around it (`BarPointer`): a menu session opens on the press, and a workspace strip
//! reorders by a press on one pill released over another.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Driver, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::bar::pointer::BarPointer;
use sill_shell_kit::prelude::*;
use std::cell::RefCell;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 320,
    height: 120,
    scale_percent: 100,
};

thread_local! {
    static HEARD: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn heard(what: &'static str) -> EventHandler<PointerEvent> {
    EventHandler::new(move |_| HEARD.with(|log| log.borrow_mut().push(what)))
}

fn pointer(prefix: &'static str) -> BarPointer {
    let (down, up, enter) = match prefix {
        "title" => ("title down", "title up", "title enter"),
        _ => ("pill down", "pill up", "pill enter"),
    };
    BarPointer {
        down: Some(heard(down)),
        up: Some(heard(up)),
        enter: Some(heard(enter)),
    }
}

#[allow(non_snake_case)]
fn Bar() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Bar,
            div { style: "display:flex; gap:20px; padding:20px",
                MenuBarItem { label: "File", pointer: pointer("title"), onclick: |_| {} }
                WorkspacePills { label: "Workspaces",
                    WorkspacePill { label: "1", current: Selection::Selected, pointer: pointer("pill"), onclick: |_| {} }
                }
            }
        }
    }
}

fn started() -> Harness {
    HEARD.with(|log| log.borrow_mut().clear());
    let mut harness = Harness::new(Bar, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(100));
    harness
}

fn log() -> Vec<&'static str> {
    HEARD.with(|log| log.borrow().clone())
}

#[test]
fn a_menu_bar_item_and_a_workspace_pill_hear_the_pointer_themselves() {
    let mut harness = started();
    let title = harness
        .centre(".ds-menu-bar-item")
        .expect("the item is laid out");
    harness.send(Input::pointer_move(title));
    harness.send(Input::click(title));
    let heard = log();
    for want in ["title enter", "title down", "title up"] {
        assert!(heard.contains(&want), "{want}: {heard:?}");
    }
    HEARD.with(|log| log.borrow_mut().clear());
    let pill = harness.centre(".ds-ws-pill").expect("the pill is laid out");
    harness.send(Input::click(pill));
    let heard = log();
    for want in ["pill down", "pill up"] {
        assert!(heard.contains(&want), "{want}: {heard:?}");
    }
}
