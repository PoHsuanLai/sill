//! The backend choice and the support table.

use casement_protocols::vocab::PlaceTarget;
use porter_core::SpaceId;

use super::{DesktopBackend, DesktopCommand, GlobalOffer, Supports, backend_for, supports};

fn offer(interfaces: &[&str]) -> GlobalOffer {
    GlobalOffer::from_globals(interfaces.iter().map(|name| (*name, 1)))
}

#[test]
fn the_globals_choose_the_backend() {
    let cases: &[(&str, &[&str], DesktopBackend)] = &[
        ("nothing offered", &[], DesktopBackend::Cosmic),
        (
            "cosmic's own protocols",
            &[
                "zcosmic_toplevel_info_v1",
                "ext_workspace_manager_v1",
                "ext_foreign_toplevel_list_v1",
            ],
            DesktopBackend::Cosmic,
        ),
        (
            "standard toplevel management",
            &[
                "ext_foreign_toplevel_list_v1",
                "zwlr_foreign_toplevel_manager_v1",
                "ext_workspace_manager_v1",
            ],
            DesktopBackend::Wlr,
        ),
        (
            "wlr beats cosmic when both are offered",
            &[
                "zwlr_foreign_toplevel_manager_v1",
                "zcosmic_toplevel_info_v1",
            ],
            DesktopBackend::Wlr,
        ),
        (
            "quire beats wlr",
            &[
                "quire_desktop_manager_v1",
                "zwlr_foreign_toplevel_manager_v1",
            ],
            DesktopBackend::Quire,
        ),
        (
            "only the list and workspaces are not enough for wlr",
            &["ext_foreign_toplevel_list_v1", "ext_workspace_manager_v1"],
            DesktopBackend::Cosmic,
        ),
    ];
    for (name, globals, want) in cases {
        assert_eq!(backend_for(&offer(globals)), *want, "{name}");
    }
}

#[test]
fn a_repeated_global_keeps_its_highest_version() {
    let both = GlobalOffer::from_globals([("wl_output", 4), ("wl_output", 2), ("wl_seat", 7)]);
    assert_eq!(both.version("wl_output"), Some(4));
    assert_eq!(both.version("wl_seat"), Some(7));
    assert_eq!(both.version("wl_shm"), None);
    assert!(both.offers("wl_seat"));
    assert!(!both.offers("wl_shm"));
}

#[test]
fn each_backend_carries_what_its_protocols_can() {
    let window = || "w1".to_owned();
    let space = || SpaceId::parse("ws-1").unwrap_or_else(|e| panic!("{e}"));
    let commands: Vec<(&str, DesktopCommand)> = vec![
        ("activate", DesktopCommand::Activate(window())),
        ("close", DesktopCommand::Close(window())),
        ("minimise", DesktopCommand::Minimise(window())),
        ("unminimise", DesktopCommand::Unminimise(window())),
        ("place", DesktopCommand::Place(window(), PlaceTarget::Left)),
        ("to space", DesktopCommand::ToSpace(window(), space())),
        ("switch space", DesktopCommand::SwitchSpace(space())),
        (
            "minimise target",
            DesktopCommand::SetMinimiseTarget {
                toplevel: window(),
                rect: (0, 0, 64, 64),
            },
        ),
    ];
    use Supports::{No, Yes};
    // Rows: activate, close, minimise, unminimise, place, to space, switch space, target.
    let table: &[(DesktopBackend, [Supports; 8])] = &[
        (
            DesktopBackend::Quire,
            [Yes, Yes, Yes, Yes, Yes, Yes, Yes, Yes],
        ),
        (DesktopBackend::Wlr, [Yes, Yes, Yes, Yes, No, No, Yes, No]),
        (
            DesktopBackend::Cosmic,
            [Yes, Yes, Yes, Yes, No, Yes, Yes, No],
        ),
    ];
    for (backend, row) in table {
        for ((name, command), want) in commands.iter().zip(row) {
            assert_eq!(supports(*backend, command), *want, "{backend:?} {name}");
        }
    }
}
