//! Which outputs a surface lives on, and which one its panel opens on.

use shell_host::OutputId;

/// A surface's rule for outputs, declared once as [`super::Surface::OUTPUTS`]. The daemon spawns
/// by it and every opener (a verb, a chord, the companion) picks the output by it, so one panel
/// never opens on different screens depending on the door it came through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputPolicy {
    /// One surface on every output, and nothing picks among them: the dock, a hot corner.
    Every,
    /// One warm surface on every output; a panel opens on the focused window's output, else the
    /// pointer's. A sheet is one too: it is mapped on the output the sheets' link latches.
    FocusedWindow,
    /// A surface on the main display only: the banners, whose one column counts and pauses once.
    Main,
}

impl OutputPolicy {
    /// Whether the surface lives on `output`, given the main display.
    pub fn spawns_on(self, output: OutputId, main: Option<OutputId>) -> bool {
        match self {
            OutputPolicy::Every | OutputPolicy::FocusedWindow => true,
            OutputPolicy::Main => main == Some(output),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OutputPolicy;
    use shell_host::OutputId;

    #[test]
    fn only_a_main_surface_leaves_the_other_outputs_alone() {
        let (one, two) = (OutputId(1), OutputId(2));
        let cases = [
            (OutputPolicy::Every, Some(one), true, true),
            (OutputPolicy::FocusedWindow, Some(one), true, true),
            (OutputPolicy::Main, Some(one), true, false),
            (OutputPolicy::Main, None, false, false),
        ];
        for (policy, main, on_one, on_two) in cases {
            assert_eq!(policy.spawns_on(one, main), on_one, "{policy:?} {main:?}");
            assert_eq!(policy.spawns_on(two, main), on_two, "{policy:?} {main:?}");
        }
    }
}
