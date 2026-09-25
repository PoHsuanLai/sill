//! An expanded card's actions (design/13 §13.3.6 "Hover"): up to two as quire's `Button{Mini}`,
//! more behind one `Options` button that opens a quire `Menu` of them all, as macOS's banner
//! "Options" does. `default` is the body's click, never a button. While the menu is open the
//! column must take the whole surface's input, so the card says so on a [`MenuSeat`] the banner
//! column provides (the center takes its whole surface anyway and provides none).

use dioxus::prelude::*;
use ds::{
    Anchor, Availability, Button, ButtonVariant, Menu, MenuEntry, MenuKind, MountedRef,
    Propagation, Trail, Trailing,
};
use sill_services::{Action, ActionKey};

use super::CardPress;

/// Whether one of a column's cards has its actions menu open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuOpen {
    #[default]
    No,
    Yes,
}

/// The column's seat for [`MenuOpen`], provided as a context by the banners.
#[derive(Debug, Clone, Copy)]
pub struct MenuSeat(pub Signal<MenuOpen>);

/// How a card lays its actions out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionLayout {
    None,
    Buttons(Vec<Action>),
    Menu(Vec<Action>),
}

/// The most actions drawn as buttons.
const BUTTONS_MAX: usize = 2;

/// How `buttons` (every action but `default`) are drawn.
pub fn action_layout(buttons: Vec<Action>) -> ActionLayout {
    match buttons.len() {
        0 => ActionLayout::None,
        n if n <= BUTTONS_MAX => ActionLayout::Buttons(buttons),
        _ => ActionLayout::Menu(buttons),
    }
}

/// The row of actions under an expanded card's text.
#[component]
pub fn CardActions(layout: ActionLayout, on_press: EventHandler<CardPress>) -> Element {
    let mut open = use_signal(|| MenuOpen::No);
    let mut anchor = use_signal(|| None::<MountedRef>);
    let seat = try_use_context::<MenuSeat>();
    let mut tell = move |state: MenuOpen| {
        open.set(state);
        if let Some(MenuSeat(mut seat)) = seat {
            seat.set(state);
        }
    };
    use_drop(move || {
        if let Some(MenuSeat(mut seat)) = seat {
            seat.set(MenuOpen::No);
        }
    });
    match layout {
        ActionLayout::None => rsx! {},
        ActionLayout::Buttons(actions) => rsx! {
            div { class: "sill-card-actions",
                for action in actions {
                    Button {
                        key: "{action.key.0}",
                        variant: ButtonVariant::Mini,
                        label: action.label.clone(),
                        propagation: Propagation::Stop,
                        onclick: move |_| on_press.call(CardPress::Action(action.key.clone())),
                    }
                }
            }
        },
        ActionLayout::Menu(actions) => {
            let entries: Vec<MenuEntry<ActionKey>> = actions
                .into_iter()
                .map(|action| MenuEntry::Item {
                    value: action.key,
                    title: action.label,
                    detail: None,
                    tile: None,
                    trail: Trail::None,
                    check: None,
                    availability: Availability::Enabled,
                })
                .collect();
            rsx! {
                div { class: "sill-card-actions",
                    Button {
                        variant: ButtonVariant::Mini,
                        label: "Options".to_owned(),
                        trailing: Some(Trailing::Caret),
                        propagation: Propagation::Stop,
                        mounted: move |event: MountedEvent| anchor.set(Some(MountedRef(event.data()))),
                        onclick: move |_| tell(MenuOpen::Yes),
                    }
                }
                if let (MenuOpen::Yes, Some(button)) = (open(), anchor()) {
                    Menu {
                        kind: MenuKind::Dropdown,
                        anchor: Anchor::Mounted(button),
                        entries,
                        onpick: move |key: ActionKey| on_press.call(CardPress::Action(key)),
                        onclose: move |()| tell(MenuOpen::No),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionLayout, action_layout};
    use sill_services::{Action, ActionKey};

    fn action(key: &str) -> Action {
        Action {
            key: ActionKey(key.to_owned()),
            label: key.to_owned(),
        }
    }

    #[test]
    fn two_are_buttons_and_more_are_a_menu() {
        let cases = [
            (vec![], ActionLayout::None),
            (vec![action("a")], ActionLayout::Buttons(vec![action("a")])),
            (
                vec![action("a"), action("b")],
                ActionLayout::Buttons(vec![action("a"), action("b")]),
            ),
            (
                vec![action("a"), action("b"), action("c")],
                ActionLayout::Menu(vec![action("a"), action("b"), action("c")]),
            ),
        ];
        for (buttons, want) in cases {
            assert_eq!(action_layout(buttons.clone()), want, "{buttons:?}");
        }
    }
}
