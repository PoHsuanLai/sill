//! The window menu as a [`MenuModel`]: which entries it has and which are dimmed, as a function
//! of the bit set the compositor sent (a bit per `WindowEntry` code, set when the entry can be
//! picked now).

use ds::prelude::*;
use sill_model::desktop::{WindowEntry, WireCode};
use sill_ui_kit::menu::MenuModel;

/// The menu of one window, opened with the compositor's `enabled` bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowMenuModel {
    pub(super) enabled: u32,
}

impl MenuModel for WindowMenuModel {
    type Pick = WindowEntry;

    fn entries(&self) -> Vec<MenuItem<WindowEntry>> {
        entries(self.enabled)
    }

    /// The menu is always the same set: an entry the window cannot take is dimmed, not left out.
    fn unavailable(&self) -> ContextUnavailable {
        ContextUnavailable::Dim
    }
}

/// Whether the bit set `enabled` offers `entry`.
pub(super) fn offered(enabled: u32, entry: WindowEntry) -> bool {
    1_u32
        .checked_shl(entry.code())
        .is_some_and(|bit| enabled & bit != 0)
}

/// The words the menu shows for `entry`, as the Mac words them.
pub(super) fn label(entry: WindowEntry) -> &'static str {
    match entry {
        WindowEntry::EnterFullScreen => "Enter Full Screen",
        WindowEntry::ExitFullScreen => "Exit Full Screen",
        WindowEntry::Left => "Left",
        WindowEntry::Right => "Right",
        WindowEntry::Top => "Top",
        WindowEntry::Bottom => "Bottom",
        WindowEntry::TopLeft => "Top Left",
        WindowEntry::TopRight => "Top Right",
        WindowEntry::BottomLeft => "Bottom Left",
        WindowEntry::BottomRight => "Bottom Right",
        WindowEntry::ReturnToPreviousSize => "Return to Previous Size",
        WindowEntry::TileLeftOfScreen => "Left of Screen",
        WindowEntry::TileRightOfScreen => "Right of Screen",
        WindowEntry::Fill => "Fill",
        WindowEntry::Centre => "Center",
    }
}

/// One entry, dimmed when `enabled` does not offer it.
fn item(enabled: u32, entry: WindowEntry) -> MenuItem<WindowEntry> {
    let availability = match offered(enabled, entry) {
        true => Availability::Enabled,
        false => Availability::Disabled,
    };
    MenuItem::new(entry, label(entry)).with_availability(availability)
}

/// A submenu of `children`; it opens when any of them can be picked.
fn submenu(
    title: &str,
    enabled: u32,
    children: Vec<MenuItem<WindowEntry>>,
) -> MenuItem<WindowEntry> {
    let any = children.iter().any(|child| match child {
        MenuItem::Item { value, .. } => offered(enabled, *value),
        _ => false,
    });
    let availability = match any {
        true => Availability::Enabled,
        false => Availability::Disabled,
    };
    MenuItem::Submenu {
        title: title.to_owned(),
        image: None,
        availability,
        children,
    }
}

/// The first entry: Exit Full Screen when the window is in full screen (the only thing the
/// compositor offers it), else Enter Full Screen (dimmed when the window cannot).
fn full_screen(enabled: u32) -> WindowEntry {
    match offered(enabled, WindowEntry::ExitFullScreen) {
        true => WindowEntry::ExitFullScreen,
        false => WindowEntry::EnterFullScreen,
    }
}

/// The entries, top to bottom.
pub(super) fn entries(enabled: u32) -> Vec<MenuItem<WindowEntry>> {
    use WindowEntry::{
        Bottom, BottomLeft, BottomRight, Centre, Fill, Left, ReturnToPreviousSize, Right,
        TileLeftOfScreen, TileRightOfScreen, Top, TopLeft, TopRight,
    };
    let each = |entries: &[WindowEntry]| -> Vec<MenuItem<WindowEntry>> {
        entries.iter().map(|entry| item(enabled, *entry)).collect()
    };
    let move_and_resize: Vec<MenuItem<WindowEntry>> = each(&[Left, Right, Top, Bottom])
        .into_iter()
        .chain([MenuItem::Separator])
        .chain(each(&[TopLeft, TopRight, BottomLeft, BottomRight]))
        .chain([MenuItem::Separator])
        .chain(each(&[ReturnToPreviousSize]))
        .collect();
    vec![
        item(enabled, full_screen(enabled)),
        submenu("Move & Resize", enabled, move_and_resize),
        submenu(
            "Full Screen Tile",
            enabled,
            each(&[TileLeftOfScreen, TileRightOfScreen]),
        ),
        item(enabled, Fill),
        item(enabled, Centre),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bit set that offers exactly `entries`.
    fn mask(entries: &[WindowEntry]) -> u32 {
        entries
            .iter()
            .fold(0, |bits, entry| bits | (1 << entry.code()))
    }

    /// (title, dimmed) of an item or submenu; `None` for a rule.
    fn shown(entry: &MenuItem<WindowEntry>) -> Option<(String, bool)> {
        match entry {
            MenuItem::Item {
                title,
                availability,
                ..
            }
            | MenuItem::Submenu {
                title,
                availability,
                ..
            } => Some((title.clone(), *availability == Availability::Disabled)),
            _ => None,
        }
    }

    fn children(entry: &MenuItem<WindowEntry>) -> &[MenuItem<WindowEntry>] {
        match entry {
            MenuItem::Submenu { children, .. } => children,
            _ => &[],
        }
    }

    /// (case, bits offered, the top level as (title, dimmed)).
    type TopCase = (&'static str, u32, [(&'static str, bool); 5]);

    #[test]
    fn the_top_level_is_full_screen_two_submenus_fill_and_centre() {
        // (case, bits offered, the top level as (title, dimmed))
        use WindowEntry::{Centre, EnterFullScreen, ExitFullScreen, Fill, Left, Right};
        let cases: [TopCase; 3] = [
            (
                "a floating window",
                mask(&[EnterFullScreen, Left, Right, Fill, Centre]),
                [
                    ("Enter Full Screen", false),
                    ("Move & Resize", false),
                    ("Full Screen Tile", true),
                    ("Fill", false),
                    ("Center", false),
                ],
            ),
            (
                "a full-screen window offers only Exit Full Screen",
                mask(&[ExitFullScreen]),
                [
                    ("Exit Full Screen", false),
                    ("Move & Resize", true),
                    ("Full Screen Tile", true),
                    ("Fill", true),
                    ("Center", true),
                ],
            ),
            (
                "a window in the dock offers nothing",
                0,
                [
                    ("Enter Full Screen", true),
                    ("Move & Resize", true),
                    ("Full Screen Tile", true),
                    ("Fill", true),
                    ("Center", true),
                ],
            ),
        ];
        for (name, bits, want) in cases {
            let got: Vec<(String, bool)> = entries(bits).iter().filter_map(shown).collect();
            let want: Vec<(String, bool)> = want
                .iter()
                .map(|(title, dimmed)| ((*title).to_owned(), *dimmed))
                .collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn move_and_resize_has_the_halves_the_quarters_and_return_between_rules() {
        let bits = mask(&[WindowEntry::Left, WindowEntry::ReturnToPreviousSize]);
        let all = entries(bits);
        let inner = children(&all[1]);
        let titles: Vec<Option<(String, bool)>> = inner.iter().map(shown).collect();
        let row = |title: &str, dimmed: bool| Some((title.to_owned(), dimmed));
        assert_eq!(
            titles,
            [
                row("Left", false),
                row("Right", true),
                row("Top", true),
                row("Bottom", true),
                None,
                row("Top Left", true),
                row("Top Right", true),
                row("Bottom Left", true),
                row("Bottom Right", true),
                None,
                row("Return to Previous Size", false),
            ]
        );
        let tile: Vec<Option<(String, bool)>> = children(&all[2]).iter().map(shown).collect();
        assert_eq!(
            tile,
            [row("Left of Screen", true), row("Right of Screen", true)]
        );
    }

    #[test]
    fn a_submenu_opens_when_any_child_can_be_picked() {
        let all = entries(mask(&[WindowEntry::TileRightOfScreen]));
        assert_eq!(shown(&all[2]), Some(("Full Screen Tile".to_owned(), false)));
        assert_eq!(shown(&all[1]), Some(("Move & Resize".to_owned(), true)));
    }

    #[test]
    fn every_entry_has_words_and_a_bit() {
        for entry in WindowEntry::ALL.iter().copied() {
            assert!(!label(entry).is_empty(), "{entry:?}");
            assert!(offered(mask(&[entry]), entry), "{entry:?}");
        }
        assert!(!offered(0, WindowEntry::Fill));
        assert!(offered(u32::MAX, WindowEntry::Fill));
    }
}
