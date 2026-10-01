//! Table tests for `effects.rs`.

use super::{EffectCtx, effect_commands};
use crate::dock::items::{DockItem, ItemKind};
use crate::dock::machine::{ActivateMode, DockEffect, Instance, TileApp, TileId};
use crate::dock::pick::DockCommand;
use sill_launcher::{AppId, DesktopAction, WindowKey};
use sill_model::apps::{
    App, AppIcon, AppIndex, DbusActivation, DesktopActionEntry, Listing, Terminal,
};
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};
use sill_model::launch::LaunchCommand;
use sill_model::pins::{DockPins, PinAt, PinEvent};
use std::path::PathBuf;

fn app(id: &str, actions: &[&str]) -> App {
    App {
        id: AppId(id.to_owned()),
        name: id.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: actions
            .iter()
            .map(|a| DesktopActionEntry {
                id: (*a).to_owned(),
                name: (*a).to_owned(),
                exec: id.to_owned(),
            })
            .collect(),
        mime_types: Vec::new(),
        categories: Vec::new(),
        keyboard_profile: None,
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

fn item(tile: u32, kind: ItemKind, app: TileApp) -> DockItem {
    DockItem {
        tile: TileId(tile),
        kind,
        app,
    }
}

fn id(s: &str) -> AppId {
    AppId(s.to_owned())
}

struct World {
    items: Vec<DockItem>,
    toplevels: Toplevels,
    index: AppIndex,
    pins: DockPins,
}

fn world() -> World {
    let window = |key: u64, app: &str, activity| Toplevel {
        key: WindowKey(key),
        app_id: app.to_owned(),
        title: String::new(),
        activity,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: Vec::new(),
        outputs: Vec::new(),
    };
    World {
        items: vec![
            item(
                1,
                ItemKind::Pinned(id("term")),
                TileApp::Active { windows: 2 },
            ),
            item(2, ItemKind::Pinned(id("files")), TileApp::NotRunning),
            item(3, ItemKind::Running(id("firefox")), TileApp::Inactive),
            item(4, ItemKind::Trash, TileApp::NotRunning),
        ],
        toplevels: Toplevels {
            windows: vec![
                window(7, "term", Activity::Active),
                window(8, "firefox", Activity::Inactive),
                window(6, "term", Activity::Inactive),
            ],
        },
        index: AppIndex {
            apps: vec![
                app("files", &[]),
                app("firefox", &[]),
                app("term", &["new-window"]),
            ],
        },
        pins: DockPins {
            pinned: vec![id("term"), id("files")],
        },
    }
}

#[test]
fn every_outward_effect_becomes_its_commands() {
    let w = world();
    let ctx = EffectCtx {
        items: &w.items,
        toplevels: &w.toplevels,
        index: &w.index,
        pins: &w.pins,
        workspace: None,
    };
    let launch = |app: &str, action: DesktopAction, uris: Vec<String>| DockCommand::Launch {
        app: id(app),
        action,
        uris,
    };
    let t = TileId;
    let cases: Vec<(&str, DockEffect, Vec<DockCommand>)> = vec![
        (
            "a click launches a pinned app",
            DockEffect::Launch {
                tile: t(2),
                instance: Instance::Existing,
            },
            vec![launch("files", DesktopAction::Default, vec![])],
        ),
        (
            "a middle click prefers new-window",
            DockEffect::Launch {
                tile: t(1),
                instance: Instance::New,
            },
            vec![launch(
                "term",
                DesktopAction::Named("new-window".to_owned()),
                vec![],
            )],
        ),
        (
            "a middle click without new-window runs the entry",
            DockEffect::Launch {
                tile: t(2),
                instance: Instance::New,
            },
            vec![launch("files", DesktopAction::Default, vec![])],
        ),
        (
            "raise the most recent window",
            DockEffect::Activate {
                tile: t(3),
                mode: ActivateMode::Raise,
            },
            vec![DockCommand::Wl(WlCommand::Activate(WindowKey(8)))],
        ),
        (
            "cycle the active app",
            DockEffect::Activate {
                tile: t(1),
                mode: ActivateMode::Cycle,
            },
            vec![DockCommand::Wl(WlCommand::Activate(WindowKey(6)))],
        ),
        (
            "reopen launches",
            DockEffect::Activate {
                tile: t(3),
                mode: ActivateMode::Reopen,
            },
            vec![launch("firefox", DesktopAction::Default, vec![])],
        ),
        (
            "the Trash opens trash:///",
            DockEffect::Activate {
                tile: t(4),
                mode: ActivateMode::Raise,
            },
            vec![launch(
                "com.system76.CosmicFiles",
                DesktopAction::Default,
                vec!["trash:///".to_owned()],
            )],
        ),
        (
            "Cmd-click shows in Files",
            DockEffect::ShowInFiles { tile: t(1) },
            vec![DockCommand::Service(LaunchCommand::ShowInFiles(id("term")))],
        ),
        (
            "a pinned tile moves",
            DockEffect::Reorder { tile: t(1), to: 1 },
            vec![DockCommand::Pins(PinEvent::Move {
                app: id("term"),
                to: 1,
            })],
        ),
        (
            "a pinned tile dropped where it was stays",
            DockEffect::Reorder { tile: t(1), to: 0 },
            vec![],
        ),
        (
            "a running tile dropped among the pins is pinned",
            DockEffect::Reorder { tile: t(3), to: 1 },
            vec![DockCommand::Pins(PinEvent::Pin {
                app: id("firefox"),
                at: PinAt::Index(1),
            })],
        ),
        (
            "a running tile dropped past the pins stays unpinned",
            DockEffect::Reorder { tile: t(3), to: 2 },
            vec![],
        ),
        (
            "dragged out: unpinned",
            DockEffect::Unpin { tile: t(2) },
            vec![DockCommand::Pins(PinEvent::Unpin(id("files")))],
        ),
        (
            "an unpinned tile dragged out stays",
            DockEffect::Unpin { tile: t(3) },
            vec![],
        ),
        ("a redraw is the dock's own", DockEffect::Redraw, vec![]),
    ];
    for (name, effect, want) in cases {
        assert_eq!(effect_commands(&effect, &ctx), want, "{name}");
    }
}
