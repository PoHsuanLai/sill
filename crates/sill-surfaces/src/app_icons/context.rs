//! What a surface's app icons depend on besides the icon itself.

use ds::Scale;

use crate::app_icons::cache::IconResolver;
use crate::app_icons::look::IconLook;

/// The style, the output scale, and the cache a shipped icon's file (`resolver.shipped_path`)
/// and a re-tinted raster (`resolver.path`) both come from.
#[derive(Debug, Clone, Copy)]
pub struct IconContext {
    pub look: IconLook,
    pub scale: Scale,
    pub resolver: IconResolver,
}
