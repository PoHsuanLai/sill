//! The tile icons of the surface on `output`, from the hooks every surface has: the icon style
//! read against the active Space's look, the output's scale, the re-tint cache and the
//! installed apps.

use ds::prelude::*;
use shell_host::OutputInfo;
use sill_services::AppsService;

use super::{IconContext, LogicalPx, TileIcons, icon_look, use_icon_resolver};
use crate::hooks::{use_appearance, use_service};
use crate::look::use_active_look;
use crate::root::use_host_scale;

/// The icons of a tile `side` points across on `output`'s surface.
pub fn use_tile_icons(output: &OutputInfo, side: LogicalPx) -> TileIcons {
    let environment = use_appearance()();
    let look = use_active_look(output);
    let appearance = environment.settings.appearance.appearance();
    let resolved = resolve(appearance, look.theme, environment.system);
    TileIcons {
        cx: IconContext {
            look: icon_look(
                &environment.settings.icons,
                &look,
                appearance.accent,
                resolved.scheme,
            ),
            scale: use_host_scale(),
            resolver: use_icon_resolver(),
        },
        index: use_service::<AppsService>()(),
        side,
    }
}
