//! The desktop menu as a [`MenuModel`]: what the entries are, as a function of what the menu
//! was opened over ([`MenuWorld`]).

use ds::prelude::*;
use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_settings::Px;
use sill_ui_kit::links::DesktopView;
use sill_ui_kit::menu::MenuModel;

/// The apps that pick a wallpaper, best first: COSMIC Settings on its Wallpaper page, then on its
/// first page. The menu offers Change Wallpaper only when the app index has one.
pub(super) const PICKERS: [&str; 2] = [
    "com.system76.CosmicSettings.Wallpaper",
    "com.system76.CosmicSettings",
];

/// How large a widget's cell is, one choice of the grid's `widgets.desktop_cell_px`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(in crate::desktop) enum CellSize {
    Small,
    Standard,
    Large,
}

impl CellSize {
    /// The side of the cell.
    pub(super) fn px(self) -> Px {
        match self {
            CellSize::Small => Px(132),
            CellSize::Standard => Px(164),
            CellSize::Large => Px(196),
        }
    }
}

/// How far apart the cells are, one choice of the grid's `widgets.desktop_gap_px`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(in crate::desktop) enum Spacing {
    Tight,
    Standard,
    Wide,
}

impl Spacing {
    /// The space between cells.
    pub(super) fn px(self) -> Px {
        match self {
            Spacing::Tight => Px(8),
            Spacing::Standard => Px(16),
            Spacing::Wide => Px(24),
        }
    }
}

/// What picking an entry of the desktop menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::desktop) enum DesktopPick {
    ChangeWallpaper,
    EditWidgets,
    Cell(CellSize),
    Gap(Spacing),
}

/// What the menu is opened over: the wallpaper picker the machine has, if any, and the grid as
/// it is now (its choices carry the check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::desktop) struct MenuWorld {
    pub(super) picker: Option<AppId>,
    pub(super) view: DesktopView,
}

impl MenuWorld {
    /// The world over `apps` with the grid at `view`.
    pub(in crate::desktop) fn of(apps: &AppIndex, view: DesktopView) -> MenuWorld {
        MenuWorld {
            picker: picker_in(apps),
            view,
        }
    }
}

/// The first of [`PICKERS`] that `apps` has.
pub(super) fn picker_in(apps: &AppIndex) -> Option<AppId> {
    PICKERS
        .iter()
        .map(|id| AppId((*id).to_owned()))
        .find(|id| apps.get(id).is_some())
}

impl MenuModel for MenuWorld {
    type Pick = DesktopPick;

    fn entries(&self) -> Vec<MenuItem<DesktopPick>> {
        entries(self)
    }
}

/// The entries, top to bottom.
pub(super) fn entries(world: &MenuWorld) -> Vec<MenuItem<DesktopPick>> {
    let wallpaper = world
        .picker
        .iter()
        .map(|_| MenuItem::new(DesktopPick::ChangeWallpaper, "Change Wallpaper\u{2026}"));
    wallpaper
        .chain([
            MenuItem::new(DesktopPick::EditWidgets, "Edit Widgets\u{2026}"),
            MenuItem::Separator,
            MenuItem::Submenu {
                title: "Show View Options".to_owned(),
                image: None,
                availability: Availability::Enabled,
                children: view_options(world.view),
            },
        ])
        .collect()
}

/// The View Options submenu: the cell's size and the spacing, each a group of choices with the
/// one in force checked.
fn view_options(view: DesktopView) -> Vec<MenuItem<DesktopPick>> {
    let sizes = CellSize::ALL.iter().map(|&size| {
        MenuItem::new(DesktopPick::Cell(size), size.label()).with_check(mark(view.cell, size.px()))
    });
    let spacing = Spacing::ALL.iter().map(|&gap| {
        MenuItem::new(DesktopPick::Gap(gap), gap.label()).with_check(mark(view.gap, gap.px()))
    });
    std::iter::once(MenuItem::Header("Widget Size".to_owned()))
        .chain(sizes)
        .chain([MenuItem::Separator, MenuItem::Header("Spacing".to_owned())])
        .chain(spacing)
        .collect()
}

/// The state mark of the choice `choice` when the setting is `now`: on when they are the same.
fn mark(now: Px, choice: Px) -> Check {
    match now == choice {
        true => Check::On,
        false => Check::Off,
    }
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use sill_launcher::AppId;
    use sill_model::apps::AppIndex;
    use sill_settings::Px;
    use sill_ui_kit::harness::fake::dock::{SETTINGS, index};
    use sill_ui_kit::links::DesktopView;

    use super::{CellSize, DesktopPick, MenuWorld, PICKERS, Spacing, entries, picker_in};

    fn world(picker: bool, cell: u16, gap: u16) -> MenuWorld {
        MenuWorld {
            picker: picker.then(|| AppId(SETTINGS.to_owned())),
            view: DesktopView {
                cell: Px(cell),
                gap: Px(gap),
            },
        }
    }

    /// The titles of `items`, a submenu's children after it in brackets, a rule as `-`.
    fn titles(items: &[MenuItem<DesktopPick>]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                MenuItem::Item { title, .. } | MenuItem::Info { title, .. } => title.clone(),
                MenuItem::Header(title) => format!("# {title}"),
                MenuItem::Separator => "-".to_owned(),
                MenuItem::Submenu {
                    title, children, ..
                } => format!("{title} [{}]", titles(children).join(", ")),
            })
            .collect()
    }

    fn checks(items: &[MenuItem<DesktopPick>]) -> Vec<(String, Check)> {
        items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Item {
                    title,
                    check: Some(check),
                    ..
                } => Some((title.clone(), *check)),
                _ => None,
            })
            .collect()
    }

    fn view_options(entries: &[MenuItem<DesktopPick>]) -> &[MenuItem<DesktopPick>] {
        entries
            .iter()
            .find_map(|item| match item {
                MenuItem::Submenu { children, .. } => Some(children.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("a submenu: {entries:?}"))
    }

    #[test]
    fn the_menu_lists_wallpaper_widgets_and_view_options_in_that_order() {
        let with = titles(&entries(&world(true, 164, 16)));
        assert_eq!(
            with,
            [
                "Change Wallpaper\u{2026}",
                "Edit Widgets\u{2026}",
                "-",
                "Show View Options [# Widget Size, Small, Standard, Large, -, # Spacing, Tight, Standard, Wide]",
            ]
        );
        let without = titles(&entries(&world(false, 164, 16)));
        assert_eq!(
            &without[..1],
            ["Edit Widgets\u{2026}"],
            "no picker, no entry"
        );
        assert_eq!(without.len(), with.len() - 1);
    }

    #[test]
    fn the_choice_in_force_is_checked_and_a_custom_value_checks_none() {
        let cases = [
            (
                "the defaults",
                164,
                16,
                Some(CellSize::Standard),
                Some(Spacing::Standard),
            ),
            (
                "small and tight",
                132,
                8,
                Some(CellSize::Small),
                Some(Spacing::Tight),
            ),
            (
                "large and wide",
                196,
                24,
                Some(CellSize::Large),
                Some(Spacing::Wide),
            ),
            ("a value the person wrote", 150, 10, None, None),
        ];
        for (name, cell, gap, size, spacing) in cases {
            let items = entries(&world(true, cell, gap));
            let on: Vec<String> = checks(view_options(&items))
                .into_iter()
                .filter(|(_, check)| *check == Check::On)
                .map(|(title, _)| title)
                .collect();
            let want: Vec<String> = size
                .map(|s| s.label().to_owned())
                .into_iter()
                .chain(spacing.map(|s| s.label().to_owned()))
                .collect();
            assert_eq!(on, want, "{name}");
        }
    }

    #[test]
    fn each_choice_picks_its_own_measure() {
        let items = entries(&world(true, 164, 16));
        let picks: Vec<DesktopPick> = view_options(&items)
            .iter()
            .filter_map(|item| match item {
                MenuItem::Item { value, .. } => Some(*value),
                _ => None,
            })
            .collect();
        let want: Vec<DesktopPick> = CellSize::ALL
            .iter()
            .map(|&s| DesktopPick::Cell(s))
            .chain(Spacing::ALL.iter().map(|&s| DesktopPick::Gap(s)))
            .collect();
        assert_eq!(picks, want);
        let px: Vec<(u16, u16)> = [
            (CellSize::Small.px(), Spacing::Tight.px()),
            (CellSize::Standard.px(), Spacing::Standard.px()),
            (CellSize::Large.px(), Spacing::Wide.px()),
        ]
        .iter()
        .map(|(c, g)| (c.0, g.0))
        .collect();
        assert_eq!(px, [(132, 8), (164, 16), (196, 24)]);
    }

    #[test]
    fn the_wallpaper_picker_is_the_best_app_the_machine_has() {
        let settings = index();
        assert_eq!(
            picker_in(&settings),
            Some(AppId(PICKERS[1].to_owned())),
            "COSMIC Settings alone"
        );
        let mut both = settings.clone();
        let mut page = both.apps[both
            .apps
            .iter()
            .position(|a| a.id.0 == SETTINGS)
            .unwrap_or(0)]
        .clone();
        page.id = AppId(PICKERS[0].to_owned());
        both.apps.push(page);
        both.apps.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(picker_in(&both), Some(AppId(PICKERS[0].to_owned())));
        assert_eq!(picker_in(&AppIndex::default()), None);
    }
}
