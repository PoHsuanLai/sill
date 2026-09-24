//! From the tracking machine's outputs to what the bar does with its one popup (design/13
//! §13.3.2 "one reusable `xdg_popup` per bar; hover switch = `xdg_popup.reposition` + content
//! swap"), and from what the popup reports back to the machine's inputs. Pure.

use super::menu_track::{Branch, ItemPath, MenuIn, MenuKey, MenuOut, Pick, Target};
use super::menus::BarMenu;
use super::pick::BarPick;

/// One thing the bar does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarEffect {
    /// Open the popup with `menu`'s contents under its title.
    Show(BarMenu),
    /// Swap the open popup's contents to `menu` and move it under that title.
    Swap(BarMenu),
    /// Close the popup.
    Hide,
    /// Run a picked item's action (after `Hide`).
    Run(BarPick),
}

/// What the popup's document reports to the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupReport {
    /// An item was picked (the ds `Menu` has already closed itself).
    Picked(BarPick),
    /// The menu asked to close: Escape, or a press on the popup outside the menu.
    Dismissed,
    /// The popup's document went away: the compositor dismissed it (an outside press ends the
    /// grab, `xdg_popup.popup_done`), or the bar closed it.
    Gone,
}

/// The machine input for `report`, and the pick it carries.
pub(crate) fn popup_input(report: PopupReport) -> (MenuIn, Option<BarPick>) {
    match report {
        PopupReport::Picked(pick) => (
            // The ds Menu names picks by value, not by place: the path is only the machine's.
            MenuIn::Release(Target::Item {
                path: ItemPath(vec![0]),
                pick: Pick::Enabled,
                branch: Branch::Leaf,
            }),
            Some(pick),
        ),
        PopupReport::Dismissed => (MenuIn::Key(MenuKey::Escape), None),
        PopupReport::Gone => (MenuIn::OutsidePress, None),
    }
}

/// The bar's effects for `outs`, with `picked` the action behind the release that produced
/// them. Highlight, submenu, adjacent-menu and timer outputs belong to the ds `Menu` inside the
/// popup, which draws its own selection; bar menus have no submenus, so the machine never asks
/// for a tick here (FINDINGS "M1 bar-ui").
pub(crate) fn effects(outs: Vec<MenuOut>, picked: Option<BarPick>) -> Vec<BarEffect> {
    let mut picked = picked;
    outs.into_iter()
        .filter_map(|out| match out {
            MenuOut::Open(id, _) => BarMenu::of(id).map(BarEffect::Show),
            MenuOut::Switch(id) => BarMenu::of(id).map(BarEffect::Swap),
            MenuOut::Close(_) => Some(BarEffect::Hide),
            MenuOut::Pick(_) => picked.take().map(BarEffect::Run),
            MenuOut::Highlight(_)
            | MenuOut::RequestTick(_)
            | MenuOut::OpenSub(_)
            | MenuOut::CloseSub
            | MenuOut::Adjacent(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BarEffect, PopupReport, effects, popup_input};
    use crate::surfaces::bar::menu_track::{MenuIn, MenuParams, MenuTrack, Target, step};
    use crate::surfaces::bar::menus::BarMenu;
    use crate::surfaces::bar::pick::BarPick;
    use shell_host::Point;
    use sill_services::Stamp;

    /// A bar input: a pointer event on a title or the bar, or what the popup reported.
    enum Input {
        Bar(MenuIn),
        Popup(PopupReport),
    }

    fn run(inputs: Vec<Input>) -> (MenuTrack, Vec<BarEffect>) {
        let params = MenuParams::new(&sill_settings::MenusSettings::default());
        inputs.into_iter().enumerate().fold(
            (MenuTrack::Closed, Vec::new()),
            |(track, mut all), (ms, input)| {
                let (input, picked) = match input {
                    Input::Bar(input) => (input, None),
                    Input::Popup(report) => popup_input(report),
                };
                let (track, outs) = step(track, input, Stamp(ms as u64 * 16), &params);
                all.extend(effects(outs, picked));
                (track, all)
            },
        )
    }

    fn press(menu: BarMenu) -> Input {
        Input::Bar(MenuIn::PressTitle(menu.id()))
    }

    fn release(menu: BarMenu) -> Input {
        Input::Bar(MenuIn::Release(Target::Title(menu.id())))
    }

    fn enter(menu: BarMenu) -> Input {
        Input::Bar(MenuIn::Move(Point { x: 0, y: 0 }, Target::Title(menu.id())))
    }

    #[test]
    fn the_bar_drives_one_popup_through_design_13s_sessions() {
        let settings = BarPick::Settings;
        let cases: Vec<(&str, Vec<Input>, Vec<BarEffect>, bool)> = vec![
            (
                "click opens and leaves it open",
                vec![press(BarMenu::Clock), release(BarMenu::Clock)],
                vec![BarEffect::Show(BarMenu::Clock)],
                true,
            ),
            (
                "a second click on the title closes",
                vec![
                    press(BarMenu::Clock),
                    release(BarMenu::Clock),
                    press(BarMenu::Clock),
                ],
                vec![BarEffect::Show(BarMenu::Clock), BarEffect::Hide],
                false,
            ),
            (
                "hover switches the one popup, no second open",
                vec![
                    press(BarMenu::Clock),
                    release(BarMenu::Clock),
                    enter(BarMenu::Battery),
                    enter(BarMenu::App),
                ],
                vec![
                    BarEffect::Show(BarMenu::Clock),
                    BarEffect::Swap(BarMenu::Battery),
                    BarEffect::Swap(BarMenu::App),
                ],
                true,
            ),
            (
                "hovering titles with nothing open does nothing",
                vec![enter(BarMenu::Battery), enter(BarMenu::Clock)],
                vec![],
                false,
            ),
            (
                "a pick closes, then runs",
                vec![
                    press(BarMenu::Clock),
                    release(BarMenu::Clock),
                    Input::Popup(PopupReport::Picked(settings.clone())),
                ],
                vec![
                    BarEffect::Show(BarMenu::Clock),
                    BarEffect::Hide,
                    BarEffect::Run(settings),
                ],
                false,
            ),
            (
                "an outside press dismisses",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    Input::Bar(MenuIn::OutsidePress),
                ],
                vec![BarEffect::Show(BarMenu::Volume), BarEffect::Hide],
                false,
            ),
            (
                "the compositor's dismissal closes the session",
                vec![
                    press(BarMenu::Network),
                    release(BarMenu::Network),
                    Input::Popup(PopupReport::Gone),
                    Input::Popup(PopupReport::Gone),
                ],
                vec![BarEffect::Show(BarMenu::Network), BarEffect::Hide],
                false,
            ),
            (
                "Escape in the menu closes",
                vec![
                    press(BarMenu::App),
                    release(BarMenu::App),
                    Input::Popup(PopupReport::Dismissed),
                ],
                vec![BarEffect::Show(BarMenu::App), BarEffect::Hide],
                false,
            ),
        ];
        for (name, inputs, want, open) in cases {
            let (track, got) = run(inputs);
            assert_eq!(got, want, "{name}");
            assert_eq!(
                matches!(track, MenuTrack::Tracking { .. }),
                open,
                "{name}: {track:?}"
            );
        }
    }
}
