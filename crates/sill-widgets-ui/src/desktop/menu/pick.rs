//! What a pick of the desktop menu asks for, as values the layer's io carries out.

use shell_host::OutputId;
use sill_launcher::AppId;
use sill_ui_kit::links::DesktopView;

use super::model::{DesktopPick, MenuWorld};

/// One thing the desktop menu asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DesktopCommand {
    /// Open the wallpaper picker app.
    OpenPicker(AppId),
    /// Open Edit Widgets on this output.
    EditWidgets(OutputId),
    /// Set the widget grid's measures.
    SetView(DesktopView),
}

/// What `pick` asks for on `output`, over `world`: nothing for Change Wallpaper when no picker
/// is installed.
pub(super) fn commands(
    pick: DesktopPick,
    world: &MenuWorld,
    output: OutputId,
) -> Vec<DesktopCommand> {
    match pick {
        DesktopPick::ChangeWallpaper => world
            .picker
            .iter()
            .cloned()
            .map(DesktopCommand::OpenPicker)
            .collect(),
        DesktopPick::EditWidgets => vec![DesktopCommand::EditWidgets(output)],
        DesktopPick::Cell(size) => vec![DesktopCommand::SetView(DesktopView {
            cell: size.px(),
            gap: world.view.gap,
        })],
        DesktopPick::Gap(spacing) => vec![DesktopCommand::SetView(DesktopView {
            cell: world.view.cell,
            gap: spacing.px(),
        })],
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;
    use sill_launcher::AppId;
    use sill_settings::Px;
    use sill_ui_kit::links::DesktopView;

    use super::super::model::{CellSize, DesktopPick, MenuWorld, Spacing};
    use super::{DesktopCommand, commands};

    fn world(picker: Option<&str>) -> MenuWorld {
        MenuWorld {
            picker: picker.map(|id| AppId(id.to_owned())),
            view: DesktopView {
                cell: Px(164),
                gap: Px(16),
            },
        }
    }

    #[test]
    fn a_pick_asks_for_its_effect_on_its_own_output() {
        let output = OutputId(7);
        let installed = world(Some("org.example.Wallpapers"));
        let cases = [
            (
                "the picker is opened",
                DesktopPick::ChangeWallpaper,
                &installed,
                vec![DesktopCommand::OpenPicker(AppId(
                    "org.example.Wallpapers".to_owned(),
                ))],
            ),
            (
                "no picker: nothing",
                DesktopPick::ChangeWallpaper,
                &world(None),
                Vec::new(),
            ),
            (
                "edit widgets on the menu's output",
                DesktopPick::EditWidgets,
                &installed,
                vec![DesktopCommand::EditWidgets(output)],
            ),
            (
                "a cell size keeps the spacing",
                DesktopPick::Cell(CellSize::Large),
                &installed,
                vec![DesktopCommand::SetView(DesktopView {
                    cell: Px(196),
                    gap: Px(16),
                })],
            ),
            (
                "a spacing keeps the cell",
                DesktopPick::Gap(Spacing::Tight),
                &installed,
                vec![DesktopCommand::SetView(DesktopView {
                    cell: Px(164),
                    gap: Px(8),
                })],
            ),
        ];
        for (name, pick, world, want) in cases {
            assert_eq!(commands(pick, world, output), want, "{name}");
        }
    }
}
