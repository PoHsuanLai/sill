//! The effects end of the bar: follow the [`SpacesBarLink`] onto this output's surface (mapped
//! while Mission Control is open on this output, unmapped once the lift has settled), publish
//! where the cards lie, and write a click back to the link.

use dioxus::prelude::*;
use ds::host::measure::{RectProbe, use_rect};
use ds::prelude::{BarSpaceKey, ImageSource, Rect, Selection, Shown, SpaceTile};
use porter_core::SpaceId;
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::apps::AppIndex;
use sill_model::desktop::target::Rect as WholeRect;
use sill_model::desktop::{
    BarLayout, BarPick, BarShow, BarSpace, bar_space_ids, bar_spaces, thumb_size, tiles_of,
};
use sill_ui_kit::links::{SpacesBarFeed, SpacesBarLink};

/// Everything the bar's handlers and render reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct Bar {
    link: SpacesBarLink,
    feed: ReadSignal<SpacesBarFeed>,
    name: Option<String>,
    /// Whether the bar is up on this output.
    pub up: Memo<BarShow>,
    /// The bar's element, once mounted.
    pub probe: RectProbe,
}

/// The bar on `output`.
pub(super) fn use_bar(link: &SpacesBarLink, output: &OutputInfo) -> Bar {
    let feed = use_watch(link.subscribe());
    let name = output.name.clone();
    let up = use_memo({
        let name = name.clone();
        move || {
            let feed = feed.read();
            feed.outputs.bar_on(feed.show, name.as_deref())
        }
    });
    let probe = use_rect();
    use_map_when_up(up);
    use_publish_layout(link, feed, up, probe, output);
    Bar {
        link: link.clone(),
        feed,
        name,
        up,
        probe,
    }
}

impl Bar {
    /// quire's word for a bar that is up or down.
    pub(super) fn shown(&self) -> Shown {
        match (self.up)() {
            BarShow::Show => Shown::Visible,
            BarShow::Hide => Shown::Hidden,
        }
    }

    /// This output's Spaces as the bar's tiles, left to right.
    pub(super) fn tiles(&self, apps: &AppIndex) -> Vec<SpaceTile> {
        let feed = self.feed.read();
        bar_spaces(&feed.view, self.name.as_deref(), apps)
            .into_iter()
            .map(|space| tile_of(&feed, self.name.as_deref(), space))
            .collect()
    }

    /// The person clicked the tile with this key.
    pub(super) fn switch(&self, key: &BarSpaceKey) {
        if let Ok(id) = SpaceId::parse(&key.0) {
            self.link.pick(BarPick::Switch(id));
        }
    }

    /// The person clicked the plus.
    pub(super) fn add(&self) {
        self.link
            .pick(BarPick::Add(self.name.clone().unwrap_or_default()));
    }
}

/// The tile for one Space: named, marked when on show, over its picture, lit when a dragged
/// window is over it.
fn tile_of(feed: &SpacesBarFeed, output: Option<&str>, space: BarSpace) -> SpaceTile {
    let current = match space.current {
        true => Selection::Selected,
        false => Selection::Unselected,
    };
    let tile = SpaceTile::new(BarSpaceKey(space.id.as_str().to_owned()), space.label)
        .current(current)
        .in_drag(feed.drops.state_of(output, &space.id));
    match output.and_then(|name| feed.pictures.png_of(name, &space.id)) {
        Some(png) => tile.thumbnail(ImageSource::png(&png)),
        None => tile,
    }
}

/// `rect`, measured on the surface, in whole logical pixels; `None` before layout gave it an area.
pub(super) fn whole(rect: Rect) -> Option<WholeRect> {
    let side = |px: f32| px.round().max(0.0) as u32;
    let (width, height) = (side(rect.size.width.0), side(rect.size.height.0));
    (width > 0 && height > 0).then(|| {
        (
            rect.origin.x.0.round() as i32,
            rect.origin.y.0.round() as i32,
            width,
            height,
        )
    })
}

/// Keep the link told where this output's cards lie while the bar is up and laid out, and that
/// there are none once it goes.
fn use_publish_layout(
    link: &SpacesBarLink,
    feed: ReadSignal<SpacesBarFeed>,
    up: Memo<BarShow>,
    probe: RectProbe,
    output: &OutputInfo,
) {
    let link = link.clone();
    let name = output.name.clone().unwrap_or_default();
    let origin = (output.logical.x, output.logical.y);
    let scale = output.scale.0;
    use_effect(move || match up() {
        _ if name.is_empty() => {}
        BarShow::Hide => link.set_layout(&name, None),
        BarShow::Show => {
            let Some(bar) = probe.rect().and_then(whole) else {
                return;
            };
            let ids = bar_space_ids(&feed.read().view, Some(&name));
            let layout = BarLayout {
                tiles: tiles_of(origin, bar, &ids),
                thumb: thumb_size(scale),
            };
            link.set_layout(&name, Some(layout));
        }
    });
}

/// Map the surface once Mission Control opens on this output. Never unmaps here: the lift plays
/// first, and [`use_unmap`] is the surface's only way down. Nothing to do without a host (a
/// test, an SSR render).
fn use_map_when_up(up: Memo<BarShow>) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(BarShow::Hide));
    use_effect(move || {
        let show = up();
        if *applied.peek() == show {
            return;
        }
        applied.set(show);
        if show == BarShow::Show
            && let Some(surface) = &surface
        {
            surface.apply(SurfaceOp::SetVisibility(Visibility::Shown));
        }
    });
}

/// The handler for the end of the lift: unmap the surface. Nothing to do without a host (a
/// test, an SSR render).
pub(super) fn use_unmap() -> EventHandler<()> {
    let surface = try_use_context::<SurfaceHandle>();
    use_hook(|| {
        EventHandler::new(move |()| {
            if let Some(surface) = &surface {
                surface.apply(SurfaceOp::SetVisibility(Visibility::Hidden));
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::whole;
    use ds::prelude::{Point, Px, Rect, Size};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect {
            origin: Point { x: Px(x), y: Px(y) },
            size: Size {
                width: Px(w),
                height: Px(h),
            },
        }
    }

    #[test]
    fn a_measured_box_is_whole_pixels_and_one_not_laid_out_is_none() {
        assert_eq!(
            whole(rect(0.4, 23.6, 1000.2, 149.5)),
            Some((0, 24, 1000, 150))
        );
        assert_eq!(whole(rect(0.0, 0.0, 0.0, 0.0)), None);
        assert_eq!(whole(rect(0.0, 0.0, 800.0, 0.0)), None);
    }
}
