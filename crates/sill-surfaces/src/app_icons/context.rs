//! What a surface's app icons depend on besides the icon itself.

use ds::Scale;

use crate::app_icons::cache::IconResolver;
use crate::app_icons::look::IconLook;
use crate::app_icons::plan::AssetDir;

/// The style, the output scale, quire's files (none when they were not found), and the cache
/// re-tinted rasters come from.
#[derive(Debug, Clone, Copy)]
pub struct IconContext<'a> {
    pub look: IconLook,
    pub scale: Scale,
    pub assets: Option<&'a AssetDir>,
    pub resolver: IconResolver,
}
