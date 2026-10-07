//! What each layout the host hands Edit Widgets added: the gallery keeps no layout, so it learns
//! that an Add landed by seeing the new placement in the next layout, never from the click itself
//! (a refused edit, a full desktop, lands nothing). The book remembers the identities it saw
//! last; the new row is brought into view.

use crate::widget::layout::WidgetLayout;
use std::collections::BTreeSet;

use crate::catalog::placement::PlacementId;

/// The gallery's memory between layouts.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Book {
    seen: Option<BTreeSet<PlacementId>>,
}

impl Book {
    /// Read `layout`: the placements that are new since the last one. The first layout read adds
    /// nothing: what was placed before the gallery opened is not new.
    pub(crate) fn read(&mut self, layout: &WidgetLayout) -> BTreeSet<PlacementId> {
        let now: BTreeSet<PlacementId> = layout.items().iter().map(|item| item.id).collect();
        let Some(before) = self.seen.replace(now.clone()) else {
            return BTreeSet::new();
        };
        now.difference(&before).copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Book;
    use crate::widget::battery::BatteryWidget;
    use crate::widget::calendar::MonthWidget;
    use crate::widget::contract::Widget;
    use crate::widget::kind::{WidgetHost, WidgetSize};
    use crate::widget::layout::{DesktopGrid, WidgetEdit, WidgetLayout, apply};

    const GRID: DesktopGrid = DesktopGrid {
        columns: 4,
        rows: 3,
    };

    fn add(
        layout: &WidgetLayout,
        kind: crate::widget::contract::WidgetKind,
        host: WidgetHost,
    ) -> WidgetLayout {
        let edit = WidgetEdit::Add {
            kind,
            size: WidgetSize::Small,
            host,
        };
        apply(layout.clone(), edit, GRID).unwrap_or_else(|_| layout.clone())
    }

    #[test]
    fn only_a_placement_new_since_the_last_layout_arrives() {
        let mut book = Book::default();
        let opened = add(
            &WidgetLayout::default(),
            BatteryWidget::kind(),
            WidgetHost::Desktop,
        );
        assert!(
            book.read(&opened).is_empty(),
            "placed before the gallery opened"
        );
        assert!(book.read(&opened).is_empty(), "the same layout again");
        let next = add(&opened, MonthWidget::kind(), WidgetHost::Tile);
        assert_eq!(book.read(&next).len(), 1);
        assert!(book.read(&next).is_empty(), "read once");
    }

    #[test]
    fn a_refused_add_lands_nothing() {
        let mut book = Book::default();
        let layout = WidgetLayout::default();
        assert!(book.read(&layout).is_empty());
        assert!(book.read(&layout).is_empty());
    }
}
