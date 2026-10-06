//! From the tracking machine's outputs to what the bar does with its one popup (design/13
//! §13.3.2 "one reusable `xdg_popup` per bar; hover switch = `xdg_popup.reposition` + content
//! swap"), and from what the popup reports back to the machine's inputs. Pure.

use ds::prelude::*;
use ds::stack::menu_track::types::{Branch, ItemPath, MenuKey, Pickable};

use super::menus::BarMenu;
use super::pick::BarPick;
use super::track::{Effect, Event, MenuId, Target};

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

/// An enabled item of the open menu: its place among the menu's choices (quire's numbering:
/// items and submenu parents, not headers, status lines or rules) and what it picks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Choice {
    pub index: usize,
    pub pick: BarPick,
}

/// Where a released button was over the open menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Landing {
    /// Over an enabled item.
    Pick(Choice),
    /// Over an enabled submenu parent, which the popup keeps open for its submenu.
    Submenu,
    /// Over nothing pickable: a status line, a disabled row, the menu's padding.
    Nowhere,
}

/// What the popup's document reports to the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupReport {
    /// An item was picked (the ds `Menu` has already closed itself).
    Picked(BarPick),
    /// The pointer came onto the choice at this index, or off every choice (ds `Menu`'s
    /// `on_hover`).
    Hovered(Option<usize>),
    /// A button came up over the menu (ds `Menu`'s `on_release`).
    Released(Landing),
    /// The menu asked to close: Escape, or a press on the popup outside the menu.
    Dismissed,
    /// The popup's document went away: the compositor dismissed it (an outside press ends the
    /// grab, `xdg_popup.popup_done`), or the bar closed it.
    Gone,
}

/// The machine input for `report`, and the pick it carries.
///
/// A release over an item reaches the machine before the ds `Menu`'s own pick (quire calls
/// `on_release` first), so the machine decides press-drag-release and click mode alike and the
/// `Picked` that follows finds the session closed. `Picked` remains for the keyboard's Enter.
/// Hover moves carry no point and every row is a leaf to the machine: the popup is another
/// surface, and its ds `Menu` times its own submenus (open delay, safe triangle), so the
/// machine never arms either. A release over a submenu parent is the machine's click on the
/// `open` menu's own title, which ends a press-drag and keeps the menu up, as the popup does.
pub(crate) fn popup_input(report: PopupReport, open: Option<MenuId>) -> (Event, Option<BarPick>) {
    match report {
        PopupReport::Picked(pick) => (Event::Release(item(0)), Some(pick)),
        PopupReport::Hovered(Some(index)) => (Event::Move(Point::default(), item(index)), None),
        PopupReport::Hovered(None) => (Event::Move(Point::default(), Target::Menu), None),
        PopupReport::Released(Landing::Pick(choice)) => {
            (Event::Release(item(choice.index)), Some(choice.pick))
        }
        PopupReport::Released(Landing::Submenu) => (
            Event::Release(open.map_or(Target::Menu, Target::Title)),
            None,
        ),
        PopupReport::Released(Landing::Nowhere) => (Event::Release(Target::Menu), None),
        PopupReport::Dismissed => (Event::Key(MenuKey::Escape), None),
        PopupReport::Gone => (Event::OutsidePress, None),
    }
}

/// The machine's name for the enabled item at `index`, a leaf whatever it holds.
fn item(index: usize) -> Target {
    Target::Item {
        path: ItemPath(vec![u16::try_from(index).unwrap_or(u16::MAX)]),
        pick: Pickable::Enabled,
        branch: Branch::Leaf,
    }
}

/// What a release over choice `index` of `entries` lands on, numbered as quire's `Menu` numbers
/// `on_hover`: items and submenu parents count, headers, status lines and rules do not.
pub(crate) fn landing(entries: &[MenuItem<BarPick>], index: usize) -> Landing {
    let entry = entries
        .iter()
        .filter(|entry| matches!(entry, MenuItem::Item { .. } | MenuItem::Submenu { .. }))
        .nth(index);
    match entry {
        Some(MenuItem::Item {
            value,
            availability: Availability::Enabled,
            ..
        }) => Landing::Pick(Choice {
            index,
            pick: value.clone(),
        }),
        Some(MenuItem::Submenu {
            availability: Availability::Enabled,
            ..
        }) => Landing::Submenu,
        _ => Landing::Nowhere,
    }
}

/// The bar's effects for `outs`, with `picked` the action behind the release that produced
/// them. Highlight, submenu, adjacent-menu and prepare outputs belong to the ds `Menu` inside the
/// popup, which draws its own selection and times its own submenus; the machine sees every row as
/// a leaf, so it never asks for them here.
pub(crate) fn effects(outs: Vec<Effect>, picked: Option<BarPick>) -> Vec<BarEffect> {
    let mut picked = picked;
    outs.into_iter()
        .filter_map(|out| match out {
            Effect::Open(id, _) => BarMenu::of(id).map(BarEffect::Show),
            Effect::Check(id) => BarMenu::of(id).map(BarEffect::Swap),
            Effect::Close(_) => Some(BarEffect::Hide),
            Effect::Pick(_) => picked.take().map(BarEffect::Run),
            Effect::Highlight(_)
            | Effect::Prepare(_)
            | Effect::OpenSub(_)
            | Effect::CloseSub
            | Effect::Adjacent(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BarEffect, Choice, Landing, PopupReport, effects, landing, popup_input};
    use crate::bar::menus::{BarMenu, TraySlot};
    use crate::bar::pick::BarPick;
    use crate::bar::track::{Event, Target, Track, step, timing};
    use ds::base::time::stamp::Stamp;
    use ds::prelude::*;
    use ds::stack::menu_track::types::MenuPhase;
    use sill_model::tray::TrayItemId;

    /// A bar input: a pointer event on a title or the bar, or what the popup reported.
    enum Input {
        Bar(Event),
        Popup(PopupReport),
    }

    fn run(inputs: Vec<Input>) -> (Track, Vec<BarEffect>) {
        let timing = timing(&sill_settings::MenusSettings::default());
        inputs.into_iter().enumerate().fold(
            (Track::closed(), Vec::new()),
            |(track, mut all), (ms, input)| {
                let (input, picked) = match input {
                    Input::Bar(input) => (input, None),
                    Input::Popup(report) => popup_input(report, track.open_menu().copied()),
                };
                let (track, outs) = step(track, input, Stamp(ms as u64 * 16), &timing);
                all.extend(effects(outs, picked));
                (track, all)
            },
        )
    }

    fn press(menu: BarMenu) -> Input {
        Input::Bar(Event::PressTitle(menu.id()))
    }

    fn release(menu: BarMenu) -> Input {
        Input::Bar(Event::Release(Target::Title(menu.id())))
    }

    fn enter(menu: BarMenu) -> Input {
        Input::Bar(Event::Move(
            Point {
                x: Px(0.0),
                y: Px(0.0),
            },
            Target::Title(menu.id()),
        ))
    }

    #[test]
    fn the_bar_drives_one_popup_through_design_13s_sessions() {
        let settings = BarPick::Settings;
        let telegram = BarMenu::Tray(TraySlot(0));
        let fcitx = BarMenu::Tray(TraySlot(1));
        let clicked = BarPick::Tray {
            item: TrayItemId(":1.7/StatusNotifierItem".to_owned()),
            entry: 3,
        };
        let cases: Vec<(&str, Vec<Input>, Vec<BarEffect>, bool)> = vec![
            (
                "click opens and leaves it open",
                vec![press(BarMenu::Volume), release(BarMenu::Volume)],
                vec![BarEffect::Show(BarMenu::Volume)],
                true,
            ),
            (
                "a second click on the title closes",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    press(BarMenu::Volume),
                ],
                vec![BarEffect::Show(BarMenu::Volume), BarEffect::Hide],
                false,
            ),
            (
                "hover switches the one popup, no second open",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    enter(BarMenu::Battery),
                    enter(BarMenu::App),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Swap(BarMenu::Battery),
                    BarEffect::Swap(BarMenu::App),
                ],
                true,
            ),
            (
                "hovering titles with nothing open does nothing",
                vec![enter(BarMenu::Battery), enter(BarMenu::Volume)],
                vec![],
                false,
            ),
            (
                "a pick closes, then runs",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    Input::Popup(PopupReport::Picked(settings.clone())),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Hide,
                    BarEffect::Run(settings.clone()),
                ],
                false,
            ),
            (
                "an outside press dismisses",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    Input::Bar(Event::OutsidePress),
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
                "press-drag-release onto an item picks it once",
                vec![
                    press(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(Some(0))),
                    Input::Popup(PopupReport::Released(Landing::Pick(Choice {
                        index: 0,
                        pick: settings.clone(),
                    }))),
                    // quire's own pick follows the release, and finds the session closed.
                    Input::Popup(PopupReport::Picked(settings.clone())),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Hide,
                    BarEffect::Run(settings.clone()),
                ],
                false,
            ),
            (
                "a tray item's menu is one more bar menu: hover switches to and from it",
                vec![
                    press(telegram),
                    release(telegram),
                    enter(fcitx),
                    enter(BarMenu::Volume),
                    enter(telegram),
                ],
                vec![
                    BarEffect::Show(telegram),
                    BarEffect::Swap(fcitx),
                    BarEffect::Swap(BarMenu::Volume),
                    BarEffect::Swap(telegram),
                ],
                true,
            ),
            (
                "a pick in a tray item's menu closes it and clicks the row",
                vec![
                    press(telegram),
                    release(telegram),
                    Input::Popup(PopupReport::Picked(clicked.clone())),
                ],
                vec![
                    BarEffect::Show(telegram),
                    BarEffect::Hide,
                    BarEffect::Run(clicked.clone()),
                ],
                false,
            ),
            (
                "a second click on a tray item's title closes its menu",
                vec![press(telegram), release(telegram), press(telegram)],
                vec![BarEffect::Show(telegram), BarEffect::Hide],
                false,
            ),
            (
                "a press-drag released over a submenu parent keeps the menu up",
                vec![
                    press(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(Some(2))),
                    Input::Popup(PopupReport::Released(Landing::Submenu)),
                ],
                vec![BarEffect::Show(BarMenu::Volume)],
                true,
            ),
            (
                "then a pick in its submenu closes and runs",
                vec![
                    press(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(Some(2))),
                    Input::Popup(PopupReport::Released(Landing::Submenu)),
                    Input::Popup(PopupReport::Picked(settings.clone())),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Hide,
                    BarEffect::Run(settings.clone()),
                ],
                false,
            ),
            (
                "a held release over no item closes without a pick",
                vec![
                    press(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(None)),
                    Input::Popup(PopupReport::Released(Landing::Nowhere)),
                ],
                vec![BarEffect::Show(BarMenu::Volume), BarEffect::Hide],
                false,
            ),
            (
                "in click mode a release over a status line keeps the menu",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(None)),
                    Input::Popup(PopupReport::Released(Landing::Nowhere)),
                ],
                vec![BarEffect::Show(BarMenu::Volume)],
                true,
            ),
            (
                "hovering from the menu onto another title still switches",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(Some(0))),
                    enter(BarMenu::Battery),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Swap(BarMenu::Battery),
                ],
                true,
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
            (
                // A real pointer enters B on its way to the click, so the hover switch shows B
                // before the press lands on B's title.
                "a click on another title shows its menu, the first one gone",
                vec![
                    press(BarMenu::Network),
                    release(BarMenu::Network),
                    enter(BarMenu::Volume),
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                ],
                vec![
                    BarEffect::Show(BarMenu::Network),
                    BarEffect::Swap(BarMenu::Volume),
                ],
                true,
            ),
            (
                "then a click on that title closes it",
                vec![
                    press(BarMenu::Network),
                    release(BarMenu::Network),
                    enter(BarMenu::Volume),
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    press(BarMenu::Volume),
                ],
                vec![
                    BarEffect::Show(BarMenu::Network),
                    BarEffect::Swap(BarMenu::Volume),
                    BarEffect::Hide,
                ],
                false,
            ),
            (
                "the control center takes part: a click on its item swaps it in",
                vec![
                    press(BarMenu::Volume),
                    release(BarMenu::Volume),
                    enter(BarMenu::ControlCenter),
                    press(BarMenu::ControlCenter),
                    release(BarMenu::ControlCenter),
                ],
                vec![
                    BarEffect::Show(BarMenu::Volume),
                    BarEffect::Swap(BarMenu::ControlCenter),
                ],
                true,
            ),
        ];
        for (name, inputs, want, open) in cases {
            let (track, got) = run(inputs);
            assert_eq!(got, want, "{name}");
            assert_eq!(
                matches!(track.phase, MenuPhase::Tracking(_)),
                open,
                "{name}: {track:?}"
            );
        }
    }

    #[test]
    fn choices_are_numbered_over_items_only() {
        let entry = |pick: BarPick, availability| {
            MenuItem::new(pick, String::new()).with_availability(availability)
        };
        let entries = vec![
            MenuItem::Header("Foot".to_owned()),
            MenuItem::Info {
                title: "line".to_owned(),
                detail: None,
            },
            entry(BarPick::Settings, Availability::Enabled),
            MenuItem::Separator,
            entry(
                BarPick::Own {
                    app_id: String::new(),
                    item: sill_model::app_menu::OwnItem::Quit,
                },
                Availability::Disabled,
            ),
            MenuItem::Submenu {
                title: "More".to_owned(),
                image: None,
                availability: Availability::Enabled,
                children: vec![entry(BarPick::PowerMenu, Availability::Enabled)],
            },
            MenuItem::Submenu {
                title: "Off".to_owned(),
                image: None,
                availability: Availability::Disabled,
                children: Vec::new(),
            },
        ];
        let pick = |index, pick| Landing::Pick(Choice { index, pick });
        let cases = [
            (0, pick(0, BarPick::Settings)),
            (1, Landing::Nowhere), // disabled
            (2, Landing::Submenu),
            (3, Landing::Nowhere), // a disabled parent
            (4, Landing::Nowhere), // past the end
        ];
        for (index, want) in cases {
            assert_eq!(landing(&entries, index), want, "choice {index}");
        }
    }
}
