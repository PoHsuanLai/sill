//! The offer and the support table.

use casement_protocols::vocab::{Assign, OverviewMode, PlaceTarget, Toggle};
use porter_core::SpaceId;

use super::{DesktopBackend, DesktopCommand, GlobalOffer, Supports, supports};

#[test]
fn a_repeated_global_keeps_its_highest_version() {
    let both = GlobalOffer::from_globals([("wl_output", 4), ("wl_output", 2), ("wl_seat", 7)]);
    assert_eq!(both.version("wl_output"), Some(4));
    assert_eq!(both.version("wl_seat"), Some(7));
    assert_eq!(both.version("wl_shm"), None);
    assert!(both.offers("wl_seat"));
    assert!(!both.offers("wl_shm"));
    assert_eq!(both.names().collect::<Vec<_>>(), ["wl_output", "wl_seat"]);
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
        (DesktopBackend::Bare, [No, No, No, No, No, No, No, No]),
    ];
    for (backend, row) in table {
        for ((name, command), want) in commands.iter().zip(row) {
            assert_eq!(supports(*backend, command), *want, "{backend:?} {name}");
        }
    }
}

#[test]
fn what_only_the_desktops_own_compositor_can_do_is_quire_only() {
    let window = || "w1".to_owned();
    let app = || "org.example.app".to_owned();
    let space = || SpaceId::parse("ws-1").unwrap_or_else(|e| panic!("{e}"));
    let commands = [
        DesktopCommand::ClearMinimiseTarget(window()),
        DesktopCommand::AssignSpaces(window(), Assign::AllSpaces),
        DesktopCommand::ActivateApp(app()),
        DesktopCommand::HideApp(app()),
        DesktopCommand::UnhideApp(app()),
        DesktopCommand::HideOtherApps(app()),
        DesktopCommand::QuitApp(app()),
        DesktopCommand::ForceQuitApp(app()),
        DesktopCommand::ReopenApp(app()),
        DesktopCommand::CreateSpace("DP-1".to_owned()),
        DesktopCommand::RemoveSpace(space()),
        DesktopCommand::MoveSpace {
            space: space(),
            position: 1,
        },
        DesktopCommand::Overview {
            mode: OverviewMode::MissionControl,
            toggle: Toggle::On,
        },
        DesktopCommand::ShowDesktop(Toggle::On),
        DesktopCommand::PickSwitcher(2),
        DesktopCommand::CancelSwitcher,
    ];
    for command in &commands {
        assert_eq!(supports(DesktopBackend::Quire, command), Supports::Yes);
        assert_eq!(supports(DesktopBackend::Wlr, command), Supports::No);
        assert_eq!(supports(DesktopBackend::Bare, command), Supports::No);
    }
}
