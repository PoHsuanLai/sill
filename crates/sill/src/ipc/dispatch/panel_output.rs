//! The one rule for which output a panel opens on: the focused window's output, else the first
//! output that has the panel's surface. The pointer's output is not known to the daemon, so the
//! focused window stands for it. A hot corner keeps its own output (that is where it was
//! tripped) and does not come through here.

use shell_host::OutputId;
use sill_model::ids::OutputName;
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::Surface;

use crate::outputs::Outputs;

/// The output panel `S` opens on.
pub(crate) fn panel_output<S: Surface>(outputs: &Outputs, services: &Services) -> Option<OutputId> {
    let focus = services.windows.state.borrow().toplevels.focused_output();
    let with_surface: Vec<(OutputId, Option<String>)> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.surface_of::<S>().is_some())
        .map(|(info, _)| (info.id, info.name.clone()))
        .collect();
    focused_or_first(&with_surface, focus.as_ref())
}

/// The output named `focus` among `candidates` (an output's id and name), else the first.
pub(crate) fn focused_or_first(
    candidates: &[(OutputId, Option<String>)],
    focus: Option<&OutputName>,
) -> Option<OutputId> {
    candidates
        .iter()
        .find(|(_, name)| focus.is_some_and(|focus| name.as_deref() == Some(focus.0.as_str())))
        .or_else(|| candidates.first())
        .map(|(id, _)| *id)
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;
    use sill_model::ids::OutputName;

    use super::focused_or_first;

    #[test]
    fn a_panel_opens_where_the_focused_window_is_else_on_the_first_output() {
        let outputs = [
            (OutputId(1), Some("DP-1".to_owned())),
            (OutputId(2), Some("HDMI-A-1".to_owned())),
            (OutputId(3), None),
        ];
        let on = |name: &str| Some(OutputName(name.to_owned()));
        let cases = [
            (
                "the focused window's output",
                on("HDMI-A-1"),
                Some(OutputId(2)),
            ),
            (
                "the first when it is on the first",
                on("DP-1"),
                Some(OutputId(1)),
            ),
            ("the first with no focused window", None, Some(OutputId(1))),
            (
                "the first for an output with no surface",
                on("eDP-1"),
                Some(OutputId(1)),
            ),
        ];
        for (name, focus, want) in cases {
            assert_eq!(focused_or_first(&outputs, focus.as_ref()), want, "{name}");
        }
        assert_eq!(
            focused_or_first(&[], on("DP-1").as_ref()),
            None,
            "no output has it"
        );
    }
}
