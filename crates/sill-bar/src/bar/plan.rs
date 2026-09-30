//! From the tracking machine's outputs to what the bar does with its one popup (design/13
//! §13.3.2 "one reusable `xdg_popup` per bar; hover switch = `xdg_popup.reposition` + content
//! swap"), and from what the popup reports back to the machine's inputs. Pure.

use ds::prelude::*;
use ds::stack::menu_track::types::{Branch, ItemPath, MenuKey, Pickable};

use super::menus::BarMenu;
use super::pick::BarPick;
use super::track::{Effect, Event, Target};

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

/// What the popup's document reports to the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupReport {
    /// An item was picked (the ds `Menu` has already closed itself).
    Picked(BarPick),
    /// The pointer came onto the choice at this index, or off every choice (ds `Menu`'s
    /// `on_hover`).
    Hovered(Option<usize>),
    /// A button came up over the menu (ds `Menu`'s `on_release`), over this enabled item or
    /// over nothing pickable.
    Released(Option<Choice>),
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
/// Hover moves carry no point: the popup is another surface, and the point only feeds the
/// safe triangle, which bar menus (no submenus) never arm.
pub(crate) fn popup_input(report: PopupReport) -> (Event, Option<BarPick>) {
    match report {
        PopupReport::Picked(pick) => (Event::Release(item(0)), Some(pick)),
        PopupReport::Hovered(Some(index)) => (Event::Move(Point::default(), item(index)), None),
        PopupReport::Hovered(None) => (Event::Move(Point::default(), Target::Menu), None),
        PopupReport::Released(Some(choice)) => {
            (Event::Release(item(choice.index)), Some(choice.pick))
        }
        PopupReport::Released(None) => (Event::Release(Target::Menu), None),
        PopupReport::Dismissed => (Event::Key(MenuKey::Escape), None),
        PopupReport::Gone => (Event::OutsidePress, None),
    }
}

/// The machine's name for the enabled leaf item at `index` (bar menus have no submenus).
fn item(index: usize) -> Target {
    Target::Item {
        path: ItemPath(vec![u16::try_from(index).unwrap_or(u16::MAX)]),
        pick: Pickable::Enabled,
        branch: Branch::Leaf,
    }
}

/// The enabled item at choice `index` of `entries`, numbered as quire's `Menu` numbers
/// `on_hover`: items and submenu parents count, headers, status lines and rules do not.
pub(crate) fn choice(entries: &[MenuItem<BarPick>], index: usize) -> Option<Choice> {
    let entry = entries
        .iter()
        .filter(|entry| matches!(entry, MenuItem::Item { .. } | MenuItem::Submenu { .. }))
        .nth(index)?;
    match entry {
        MenuItem::Item {
            value,
            availability: Availability::Enabled,
            ..
        } => Some(Choice {
            index,
            pick: value.clone(),
        }),
        _ => None,
    }
}

/// The bar's effects for `outs`, with `picked` the action behind the release that produced
/// them. Highlight, submenu, adjacent-menu and timer outputs belong to the ds `Menu` inside the
/// popup, which draws its own selection; bar menus have no submenus, so the machine never asks
/// for a tick here.
pub(crate) fn effects(outs: Vec<Effect>, picked: Option<BarPick>) -> Vec<BarEffect> {
    let mut picked = picked;
    outs.into_iter()
        .filter_map(|out| match out {
            Effect::Open(id, _) => BarMenu::of(id).map(BarEffect::Show),
            Effect::Check(id) => BarMenu::of(id).map(BarEffect::Swap),
            Effect::Close(_) => Some(BarEffect::Hide),
            Effect::Pick(_) => picked.take().map(BarEffect::Run),
            Effect::Highlight(_)
            | Effect::RequestTick(_)
            | Effect::OpenSub(_)
            | Effect::CloseSub
            | Effect::Adjacent(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BarEffect, Choice, PopupReport, choice, effects, popup_input};
    use crate::bar::menus::BarMenu;
    use crate::bar::pick::BarPick;
    use crate::bar::track::{Event, Target, Track, step, timing};
    use ds::prelude::*;
    use ds::stack::menu_track::types::MenuPhase;
    use sill_model::units::Stamp;

    /// A bar input: a pointer event on a title or the bar, or what the popup reported.
    enum Input {
        Bar(Event),
        Popup(PopupReport),
    }

    fn run(inputs: Vec<Input>) -> (Track, Vec<BarEffect>) {
        let timing = timing(&sill_settings::MenusSettings::default());
        inputs.into_iter().enumerate().fold(
            (Track::new(timing), Vec::new()),
            |(track, mut all), (ms, input)| {
                let (input, picked) = match input {
                    Input::Bar(input) => (input, None),
                    Input::Popup(report) => popup_input(report),
                };
                let (track, outs) = step(track, input, Stamp(ms as u64 * 16));
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
                    Input::Popup(PopupReport::Released(Some(Choice {
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
                "a held release over no item closes without a pick",
                vec![
                    press(BarMenu::Volume),
                    Input::Popup(PopupReport::Hovered(None)),
                    Input::Popup(PopupReport::Released(None)),
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
                    Input::Popup(PopupReport::Released(None)),
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
        use ds::prelude::*;
        let entry = |pick: BarPick, availability| MenuItem::Item {
            value: pick,
            title: String::new(),
            image: None,
            key: None,
            check: None,
            availability,
        };
        let entries = vec![
            MenuItem::Header("Foot".to_owned()),
            MenuItem::Info {
                title: "line".to_owned(),
                detail: None,
            },
            entry(BarPick::Settings, Availability::Enabled),
            MenuItem::Separator,
            entry(BarPick::Quit(Vec::new()), Availability::Disabled),
        ];
        let cases = [
            (0, Some(BarPick::Settings)),
            (1, None), // disabled
            (2, None), // past the end
        ];
        for (index, want) in cases {
            assert_eq!(
                choice(&entries, index).map(|c| c.pick),
                want,
                "choice {index}"
            );
        }
    }
}
