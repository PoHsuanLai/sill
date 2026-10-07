//! How dense a `MonthGrid` is drawn (design/04-COMPONENTS.md section 39). A small
//! desktop widget is one 164 px cell padded 12, so 140 px of content, and the regular grid
//! (seven 32 px columns, 224 wide) does not fit it; the compact density does. A caller cannot
//! restyle the grid (its classes are quire's), so the density is a prop, and by default it
//! follows the `WidgetFrame` the grid sits in.

use crate::widget::kind::WidgetSize;
use ds::base::word::Word;

/// The density a caller asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MonthDensity {
    /// Follow the enclosing `WidgetFrame`: compact in a `Small` or `Medium` one (a medium card's
    /// content box is 140 tall, too short for six regular weeks), regular in a `Large` one and
    /// outside any frame.
    #[default]
    Auto,
    /// Seven 32 px columns, whatever encloses it.
    Regular,
    /// Fills its box, whatever encloses it: seven even columns (20 px in a small frame's 140) and
    /// the weeks sharing the height (20 px rows where the box sets no height); never week numbers.
    Compact,
}

/// The density drawn, once `Auto` is resolved: what `data-density` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub(crate) enum Drawn {
    /// The regular grid.
    Regular,
    /// The compact grid.
    Compact,
}

impl MonthDensity {
    /// The density drawn inside `frame`, the enclosing `WidgetFrame`'s size if any.
    pub(crate) fn resolve(self, frame: Option<WidgetSize>) -> Drawn {
        match (self, frame) {
            (MonthDensity::Regular, _) => Drawn::Regular,
            (MonthDensity::Compact, _)
            | (MonthDensity::Auto, Some(WidgetSize::Small | WidgetSize::Medium)) => Drawn::Compact,
            (MonthDensity::Auto, Some(WidgetSize::Large) | None) => Drawn::Regular,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Drawn, MonthDensity};
    use crate::widget::kind::WidgetSize;

    #[test]
    fn auto_follows_the_frame_and_the_others_force_it() {
        let cases = [
            (MonthDensity::Auto, None, Drawn::Regular),
            (MonthDensity::Auto, Some(WidgetSize::Small), Drawn::Compact),
            (MonthDensity::Auto, Some(WidgetSize::Medium), Drawn::Compact),
            (MonthDensity::Auto, Some(WidgetSize::Large), Drawn::Regular),
            (
                MonthDensity::Regular,
                Some(WidgetSize::Small),
                Drawn::Regular,
            ),
            (MonthDensity::Compact, None, Drawn::Compact),
            (
                MonthDensity::Compact,
                Some(WidgetSize::Large),
                Drawn::Compact,
            ),
        ];
        for (asked, frame, want) in cases {
            assert_eq!(asked.resolve(frame), want, "{asked:?} in {frame:?}");
        }
    }
}
