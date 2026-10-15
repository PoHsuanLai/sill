//! What the eyedropper decides, pure: which picture this output's surface draws, how big the
//! surface is, and the style text that places the magnifier.

use shell_host::OutputInfo;
use sill_model::capture_pick::{AskId, CELL, Loupe, SIDE};
use sill_model::desktop::OutputShot;
use sill_ui_kit::links::EyedropFeed;

/// The colour question and the picture of its output that the surface named `name` draws: the
/// feed's picture of that output, unless the person has answered that question here already.
pub(super) fn shown_on(
    feed: &EyedropFeed,
    name: Option<&str>,
    answered: Option<AskId>,
) -> Option<(AskId, OutputShot)> {
    let frozen = feed.frozen.as_ref()?;
    let shot = frozen.shots.get(name?)?;
    (answered != Some(frozen.id)).then(|| (frozen.id, shot.clone()))
}

/// The surface's size in logical pixels, none when the output reports none.
pub(super) fn surface_of(output: &OutputInfo) -> Option<(u32, u32)> {
    let side = |extent: i32| u32::try_from(extent).ok().filter(|&extent| extent > 0);
    Some((side(output.logical.width)?, side(output.logical.height)?))
}

/// The magnifier's box: where it is and how big.
pub(super) fn loupe_style(loupe: &Loupe) -> String {
    format!(
        "left:{}px;top:{}px;width:{SIDE}px;height:{SIDE}px;",
        loupe.left, loupe.top
    )
}

/// The enlarged picture inside the magnifier.
pub(super) fn zoom_style(loupe: &Loupe) -> String {
    format!(
        "left:{}px;top:{}px;width:{}px;height:{}px;",
        loupe.picture_left, loupe.picture_top, loupe.picture_width, loupe.picture_height
    )
}

/// The ring round the centre pixel's cell.
pub(super) fn cell_style() -> String {
    let at = (SIDE - CELL) / 2;
    format!("left:{at}px;top:{at}px;width:{CELL}px;height:{CELL}px;")
}
