//! The effects end of the bar: follow the [`SpacesBarLink`] onto this output's surface (mapped
//! while Mission Control is open on this output, unmapped once the lift has settled), publish
//! where the cards lie, and write a click back to the link.

use dioxus::prelude::*;
use ds::prelude::{
    BarLayout as DrawnBar, BarSpaceKey, DropState, ImageSource, Rect, Scheme, Selection, Shown,
    SpaceLook, SpaceTile, resolve,
};
use porter_core::SpaceId;
use shell_host::{OutputInfo, SurfaceHandle, Visibility, use_watch};
use sill_model::apps::AppIndex;
use sill_model::desktop::target::Rect as WholeRect;
use sill_model::desktop::{
    BarLayout, BarPick, BarShow, BarSpace, BarTile, DesktopView, bar_spaces, thumb_size,
};
use sill_model::ids::OutputName;
use sill_services::WallpaperService;
use sill_ui_kit::hooks::{use_appearance, use_service};
use sill_ui_kit::links::{SpacesBarFeed, SpacesBarLink};

use crate::wallpaper::url_for;

/// Everything the bar's handlers and render reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct Bar {
    link: SpacesBarLink,
    feed: ReadSignal<SpacesBarFeed>,
    /// The desktop, for its Spaces and their windows.
    desktop: ReadSignal<DesktopView>,
    name: Option<String>,
    /// Whether the bar is up on this output.
    pub up: Memo<BarShow>,
    /// quire's `on_layout`: keeps where the bar drew its parts.
    pub on_layout: EventHandler<DrawnBar>,
    /// The `file://` URL of this output's wallpaper, once it has one.
    pub wallpaper: Memo<Option<String>>,
}

/// The bar on `output`.
pub(super) fn use_bar(link: &SpacesBarLink, output: &OutputInfo) -> Bar {
    let feed = use_watch(link.subscribe());
    let desktop = use_watch(link.subscribe_desktop());
    let name = output.name.clone();
    let up = use_memo({
        let name = name.clone();
        move || {
            let feed = feed.read();
            feed.outputs.bar_on(feed.show, name.as_deref())
        }
    });
    use_map_when_up(up);
    let on_layout = use_publish_layout(link, up, output);
    let wallpaper = use_wallpaper_url(output.name.clone());
    Bar {
        link: link.clone(),
        feed,
        desktop,
        name,
        up,
        on_layout,
        wallpaper,
    }
}

/// The wallpaper this output shows in the scheme the desktop resolves to, as the wallpaper surface
/// itself picks it.
fn use_wallpaper_url(name: Option<String>) -> Memo<Option<String>> {
    let wallpapers = use_service::<WallpaperService>();
    let environment = use_appearance();
    use_memo(move || {
        let environment = environment();
        let appearance = environment.settings.appearance.appearance();
        let scheme: Scheme =
            resolve(appearance, SpaceLook::default().theme, environment.system).scheme;
        let on = name.clone().map(OutputName);
        url_for(&wallpapers(), on.as_ref(), scheme)
    })
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
        let wallpaper = (self.wallpaper)();
        bar_spaces(&self.desktop.read(), self.name.as_deref(), apps)
            .into_iter()
            .map(|space| tile_of(&feed, self.name.as_deref(), wallpaper.as_deref(), space))
            .collect()
    }

    /// The plus: lit while a dragged window is over it.
    pub(super) fn add_drop(&self) -> DropState {
        self.feed.read().drops.new_state_of(self.name.as_deref())
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

/// The tile for one Space: named, marked when on show, over its picture and the output's
/// wallpaper (`wallpaper`, a `file://` URL), lit when a dragged window is over it.
fn tile_of(
    feed: &SpacesBarFeed,
    output: Option<&str>,
    wallpaper: Option<&str>,
    space: BarSpace,
) -> SpaceTile {
    let current = match space.current {
        true => Selection::Selected,
        false => Selection::Unselected,
    };
    let tile = SpaceTile::new(BarSpaceKey(space.id.as_str().to_owned()), space.label)
        .current(current)
        .in_drag(feed.drops.state_of(output, &space.id));
    let tile = match wallpaper {
        Some(url) => tile.backdrop(ImageSource(url.to_owned())),
        None => tile,
    };
    match output.and_then(|name| feed.pictures.png_of(name, &space.id)) {
        Some(png) => tile.thumbnail(ImageSource::png(&png)),
        None => tile,
    }
}

/// `rect`, in whole logical pixels, moved to the desktop by the surface's `origin`; `None` before
/// layout gave it an area.
fn whole(origin: (i32, i32), rect: Rect) -> Option<WholeRect> {
    let side = |px: f32| px.round().max(0.0) as u32;
    let (width, height) = (side(rect.size.width.0), side(rect.size.height.0));
    (width > 0 && height > 0).then(|| {
        (
            origin.0 + rect.origin.x.0.round() as i32,
            origin.1 + rect.origin.y.0.round() as i32,
            width,
            height,
        )
    })
}

/// What quire's bar drew, as the layout the link carries: the cards (a collapsed bar draws none)
/// and the plus, on a surface whose top-left is `origin` in the desktop's, with the size a
/// picture is asked for at `scale`.
fn published(drawn: &DrawnBar, origin: (i32, i32), scale: u32) -> BarLayout {
    let tiles = drawn
        .spaces
        .iter()
        .filter_map(|space| {
            let rect = space.card.and_then(|card| whole(origin, card))?;
            let id = SpaceId::parse(&space.key.0).ok()?;
            Some(BarTile { space: id, rect })
        })
        .collect();
    BarLayout {
        tiles,
        thumb: thumb_size(scale),
        add: drawn.add.and_then(|add| whole(origin, add)),
    }
}

/// Keep the link told where this output's cards and plus lie while the bar is up and has been
/// laid out, and that there are none once it goes. Returns quire's `on_layout` handler.
fn use_publish_layout(
    link: &SpacesBarLink,
    up: Memo<BarShow>,
    output: &OutputInfo,
) -> EventHandler<DrawnBar> {
    let link = link.clone();
    let name = output.name.clone().unwrap_or_default();
    let origin = (output.logical.x, output.logical.y);
    let scale = output.scale.0;
    let mut drawn = use_signal(|| None::<DrawnBar>);
    use_effect(move || match up() {
        _ if name.is_empty() => {}
        BarShow::Hide => link.set_layout(&name, None),
        BarShow::Show => {
            if let Some(drawn) = drawn() {
                link.set_layout(&name, Some(published(&drawn, origin, scale)));
            }
        }
    });
    use_hook(|| {
        EventHandler::new(move |layout: DrawnBar| {
            if drawn.peek().as_ref() != Some(&layout) {
                drawn.set(Some(layout));
            }
        })
    })
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
            surface.set_visibility(Visibility::Shown);
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
                surface.set_visibility(Visibility::Hidden);
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{published, whole};
    use ds::prelude::{BarLayout as DrawnBar, BarSpaceKey, Point, Px, Rect, Size, SpaceRects};
    use sill_model::desktop::BarTile;

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
    fn a_measured_box_is_whole_pixels_on_the_desktop_and_one_not_laid_out_is_none() {
        assert_eq!(
            whole((1920, 0), rect(0.4, 23.6, 1000.2, 149.5)),
            Some((1920, 24, 1000, 150))
        );
        assert_eq!(whole((0, 0), rect(0.0, 0.0, 0.0, 0.0)), None);
        assert_eq!(whole((0, 0), rect(0.0, 0.0, 800.0, 0.0)), None);
    }

    #[test]
    fn the_drawn_cards_and_plus_are_moved_by_the_surfaces_origin_and_a_collapsed_card_is_left_out()
    {
        let space = |key: &str, card| SpaceRects {
            key: BarSpaceKey(key.to_owned()),
            tile: rect(0.0, 0.0, 168.0, 140.0),
            card,
        };
        let drawn = DrawnBar {
            spaces: vec![
                space("ws-1", Some(rect(4.0, 12.0, 160.0, 100.0))),
                space("ws-2", None),
            ],
            add: Some(rect(952.0, 8.0, 28.0, 28.0)),
        };
        let layout = published(&drawn, (1920, 0), 240);
        let ids: Vec<_> = layout.tiles.iter().map(|t: &BarTile| t.rect).collect();
        assert_eq!(ids, [(1924, 12, 160, 100)]);
        assert_eq!(layout.add, Some((2872, 8, 28, 28)));
        assert_eq!(layout.thumb, (320, 200));
    }
}
