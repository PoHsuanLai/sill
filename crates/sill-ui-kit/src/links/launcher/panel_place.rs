//! The numbers the panel's place comes from, resolved by the panel from its tokens and read by
//! the catcher, which has no token scope of its own (it draws nothing) but must leave the panel's
//! rectangle out of its input region.

/// The resolved placement tokens, in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelPlace {
    /// The bar's height.
    pub bar_height: u16,
    /// The least space between the bar and the panel.
    pub top_bar_gap: u16,
    /// How far above the 40% line the panel sits.
    pub top_centre_bias: u16,
}
