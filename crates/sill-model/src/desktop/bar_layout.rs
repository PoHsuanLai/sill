//! Where the bar's thumbnails lie, in the desktop's logical pixels, and the commands that keep
//! the compositor's drop targets equal to them. Pure; the bar's surface measures itself and
//! publishes a [`BarLayout`], the daemon sends [`target_commands`].
//!
//! Where the cards and the plus lie is not computed here: quire's `SpacesBar` reports its drawn
//! rectangles (`on_layout`) and the surface moves them onto the desktop.

use std::collections::BTreeMap;

use porter_core::SpaceId;

use super::DesktopCommand;
use super::target::Rect;

/// The thumbnail card's width in logical pixels (quire's `.ds-spaces-bar-thumb`; it sizes the
/// picture asked for, not where the card lies).
pub const THUMB_W: u32 = 160;
/// The thumbnail card's height in logical pixels.
pub const THUMB_H: u32 = 100;
/// The room an expanded bar needs below its top edge: padding, card, gap, name, padding.
pub const BAR_EXPANDED_PX: u32 = 150;

/// One Space's drop target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarTile {
    /// The Space.
    pub space: SpaceId,
    /// Its card, in the desktop's logical pixels.
    pub rect: Rect,
}

/// The bar of one output once laid out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarLayout {
    /// The cards, left to right.
    pub tiles: Vec<BarTile>,
    /// The size in device pixels a picture of a Space is asked for at: the card at the output's
    /// scale.
    pub thumb: (u32, u32),
    /// The plus that makes a new Space, in the desktop's logical pixels.
    pub add: Option<Rect>,
}

/// Every laid-out bar, by output name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarLayouts(pub BTreeMap<String, BarLayout>);

/// The card size in device pixels at `scale` (a hundred and twentieth of a unit, as the host
/// states it: 120 is 1x).
pub fn thumb_size(scale: u32) -> (u32, u32) {
    let at = |logical: u32| (logical * scale).div_ceil(120).max(1);
    (at(THUMB_W), at(THUMB_H))
}

/// The commands that turn the targets `sent` into `now`: for each output, a set of its cards when
/// they are new or moved (an empty set, which clears, once the output has none), then the plus's
/// rectangle when that changed (`None`, which removes it, once it is gone). Outputs in order of
/// name; equal inputs give none.
pub fn target_commands(sent: &BarLayouts, now: &BarLayouts) -> Vec<DesktopCommand> {
    let layout =
        |layouts: &BarLayouts, output: &str| layouts.0.get(output).cloned().unwrap_or_default();
    let outputs: std::collections::BTreeSet<&String> = sent.0.keys().chain(now.0.keys()).collect();
    outputs
        .into_iter()
        .flat_map(|output| {
            let (old, new) = (layout(sent, output), layout(now, output));
            let cards = (old.tiles != new.tiles).then(|| DesktopCommand::SetSpaceTargets {
                output: output.clone(),
                targets: new.tiles.into_iter().map(|t| (t.space, t.rect)).collect(),
            });
            let plus = (old.add != new.add).then(|| DesktopCommand::SetNewSpaceTarget {
                output: output.clone(),
                rect: new.add,
            });
            cards.into_iter().chain(plus)
        })
        .collect()
}
