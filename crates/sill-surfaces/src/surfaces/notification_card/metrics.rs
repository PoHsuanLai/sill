//! The card's geometry from `notifications.*` (design/22 §3.12), handed to its stylesheet as
//! custom properties on the card's own slot (lengths only: the markup lint allows those; every
//! colour, radius, type size and duration stays a quire token in the sheet).

use sill_settings::NotificationsSettings;

use super::body::LineChars;

/// The gap between the icon and the text, px: the sheet's `var(--s-10)`.
const ICON_GAP: u16 = 10;

/// The card's lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardMetrics {
    pub width: u16,
    pub min_height: u16,
    pub padding: u16,
    pub icon: u16,
    pub close: u16,
    pub group_offset: u16,
}

impl CardMetrics {
    /// From the settings.
    pub fn new(n: &NotificationsSettings) -> CardMetrics {
        CardMetrics {
            width: n.banner_width_px.0,
            min_height: n.banner_min_height_px.0,
            padding: n.banner_padding_px.0,
            icon: n.icon_px.0,
            close: n.close_button_px.0,
            group_offset: n.group_offset_px.0,
        }
    }

    /// The custom properties the sheet reads.
    pub fn style_attr(&self) -> String {
        format!(
            "--sill-card-w:{}px;--sill-card-min-h:{}px;--sill-card-pad:{}px;--sill-card-icon:{}px;\
             --sill-card-close:{}px;--sill-card-layer:{}px;",
            self.width, self.min_height, self.padding, self.icon, self.close, self.group_offset
        )
    }

    /// About how many body characters fit a line.
    pub fn line_chars(&self) -> LineChars {
        LineChars::of(self.width, self.padding, self.icon, ICON_GAP)
    }
}

#[cfg(test)]
mod tests {
    use super::CardMetrics;
    use sill_settings::NotificationsSettings;

    #[test]
    fn the_defaults_are_design_13s() {
        let m = CardMetrics::new(&NotificationsSettings::default());
        assert_eq!(
            m.style_attr(),
            "--sill-card-w:360px;--sill-card-min-h:64px;--sill-card-pad:12px;--sill-card-icon:32px;\
             --sill-card-close:18px;--sill-card-layer:4px;"
        );
    }
}
