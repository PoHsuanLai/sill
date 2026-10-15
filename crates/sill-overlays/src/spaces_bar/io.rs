//! The effects end of the bar: follow the [`SpacesBarLink`] onto this output's surface (mapped
//! while Mission Control is open, unmapped once the leave has settled), play quire's presence for
//! the slide, and write a click back to the link.

use dioxus::prelude::*;
use ds::motion::anim::Anim;
use ds::motion::presence::{
    Exit,
    spec::PresenceSpec,
    use_presence::{Presented, use_presence},
};
use ds::prelude::Shown;
use shell_host::{OutputInfo, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_model::apps::AppIndex;
use sill_model::desktop::{BarPick, BarShow, BarSpace, bar_spaces};
use sill_ui_kit::links::{SpacesBarFeed, SpacesBarLink};

/// Everything the bar's handlers and render reach. Clone is cheap.
#[derive(Clone)]
pub(super) struct Bar {
    link: SpacesBarLink,
    feed: ReadSignal<SpacesBarFeed>,
    name: Option<String>,
    /// Where the slide is, for the render.
    pub presented: Presented,
}

/// The bar on `output`.
pub(super) fn use_bar(link: &SpacesBarLink, output: &OutputInfo) -> Bar {
    let feed = use_watch(link.subscribe());
    let presented = use_presence(
        shown_of(feed.read().show),
        PresenceSpec {
            enter: Anim::SheetIn,
            exit: Exit::SheetOut,
        },
        Some(use_unmap()),
    );
    use_map_when_up(feed);
    Bar {
        link: link.clone(),
        feed,
        name: output.name.clone(),
        presented,
    }
}

impl Bar {
    /// This output's Spaces, left to right.
    pub(super) fn spaces(&self, apps: &AppIndex) -> Vec<BarSpace> {
        bar_spaces(&self.feed.read().view, self.name.as_deref(), apps)
    }

    /// The person clicked a Space.
    pub(super) fn switch(&self, space: &BarSpace) {
        self.link.pick(BarPick::Switch(space.id.clone()));
    }

    /// The person clicked the plus.
    pub(super) fn add(&self) {
        self.link
            .pick(BarPick::Add(self.name.clone().unwrap_or_default()));
    }
}

/// quire's word for a bar that is up or down.
fn shown_of(show: BarShow) -> Shown {
    match show {
        BarShow::Show => Shown::Visible,
        BarShow::Hide => Shown::Hidden,
    }
}

/// Map the surface once Mission Control opens. Never unmaps here: the leave plays first, and
/// [`use_unmap`] is the surface's only way down. Nothing to do without a host (a test, an SSR
/// render).
fn use_map_when_up(feed: ReadSignal<SpacesBarFeed>) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(BarShow::Hide));
    use_effect(move || {
        let show = feed.read().show;
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

/// The handler for the presence's end of the leave: unmap the surface. Nothing to do without a
/// host (a test, an SSR render).
fn use_unmap() -> EventHandler<()> {
    let surface = try_use_context::<SurfaceHandle>();
    use_hook(|| {
        EventHandler::new(move |()| {
            if let Some(surface) = &surface {
                surface.apply(SurfaceOp::SetVisibility(Visibility::Hidden));
            }
        })
    })
}
