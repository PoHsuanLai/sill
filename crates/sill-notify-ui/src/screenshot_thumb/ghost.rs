//! The drag icon: quire's `ShotGhost` (the card at 120 wide, .8 opaque) on shell-host's
//! drag-icon surface. That surface is a document of its own, with none of the thumbnail's root
//! contexts, so the appearance it is drawn in travels as its props ([`GhostLook`]).

use dioxus::prelude::*;
use ds::components::content::image_source::ImageSize;
use ds::prelude::*;
use ds_settings::Environment;
use shell_host::{DragIcon, HostRoot, Size};
use sill_shell_kit::prelude::*;

use super::picture::ThumbPicture;

/// The ghost's width (quire's `ShotGhost`).
const GHOST_WIDTH: u32 = 120;

/// The mat round the ghost's picture, and the picture box's tallest shape (16:10), as quire
/// draws the card: the icon surface is as tall as the tallest ghost.
const GHOST_MAT: u32 = 4;

/// The appearance the ghost is drawn in: the thumbnail's own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GhostLook {
    pub appearance: Appearance,
    pub system: SystemPrefs,
    pub look: SpaceLook,
}

impl GhostLook {
    /// The look of a root drawn from `environment` on a Space of `look`.
    pub fn of(environment: &Environment, look: SpaceLook) -> Self {
        GhostLook {
            appearance: environment.settings.appearance.appearance(),
            system: environment.system,
            look,
        }
    }
}

/// The ghost's props.
#[derive(Debug, Clone, PartialEq)]
struct Ghost {
    image: ImageSource,
    size: ImageSize,
    look: GhostLook,
}

/// The icon surface's size: the ghost at its tallest.
pub fn ghost_size() -> Size {
    let inner = GHOST_WIDTH - 2 * GHOST_MAT;
    Size {
        width: GHOST_WIDTH as i32,
        height: (inner * 10 / 16 + 2 * GHOST_MAT) as i32,
    }
}

/// `picture`'s ghost as a drag icon, in `look`.
pub fn ghost_icon(picture: &ThumbPicture, look: GhostLook) -> DragIcon {
    let ghost = Ghost {
        image: picture.image.clone(),
        size: picture.size,
        look,
    };
    DragIcon::Root {
        root: HostRoot::with_props(ghost_root, ghost),
        size: ghost_size(),
    }
}

/// The icon document: a transparent Toast root holding the ghost.
fn ghost_root(ghost: Ghost) -> Element {
    ds_blitz::provide_host();
    let look = ghost.look;
    rsx! {
        Ds {
            sheet: Some(sill_shell_kit::stylesheet()),
            appearance: look.appearance,
            system: look.system,
            look: look.look,
            material: Material::Toast,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            ShotGhost { image: ghost.image, size: ghost.size }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ghost_size;

    #[test]
    fn the_icon_is_the_ghost_at_its_tallest() {
        let size = ghost_size();
        assert_eq!((size.width, size.height), (120, 78));
    }
}
