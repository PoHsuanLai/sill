//! WidgetFrame: the card a widget is drawn on (design/04-COMPONENTS.md "Widgets"; design/20
//! section 1.14, design/22 section 3.20). The frame owns the corner, the
//! padding, the footprint and the title row, so a widget draws only its content and writes no
//! CSS for any of them.
//!
//! On the desktop the card is the `Widget` material's plate inside a transparent `Widget` scope,
//! as a notification's plate is: the plate paints the tint, hairline and drop, and the scope
//! carries the material's tokens to the widget's content. In the notification center it is a
//! tile on the center's own Popover, as a `ModuleTile` is. Both take their footprint from
//! `--widget-cell` and `--widget-gap` (`WidgetMetrics`).

use crate::widget::contract::WidgetKind;
use crate::widget::kind::{CardTint, Lift, WidgetHost, WidgetSize, WidgetTitle};
use crate::widget::scope::use_frame_provider;
use dioxus::prelude::*;
use ds::base::vocab::Shown;
use ds::base::word::Word;
use ds::components::content::text_runs::text;
use ds::motion::anim::Anim;
use ds::motion::presence::Exit;
use ds::motion::presence::spec::PresenceSpec;
use ds::motion::presence::use_presence::{Presented, use_presence};
use ds::root::chrome::RootChrome;
use ds::root::common::Common;
use ds::root::surface::Surface;
use ds::style::appearance::material::Material;
use ds::style::icon::render::{Glyph, IconSize};

/// `children` on a widget's card, `size` on the grid unit, for `host`, tinted with `tint` (the
/// Space's by default; a tile never lays a second gradient, `CardTint::on`). `kind` writes
/// `data-widget` (a `WidgetCard` passes its widget's kind). `lift` picks the card up while a
/// host moves it.
/// `title` draws a glyph and a name above the content (the Batteries and Clock widgets take
/// none: their content fills the card). `common.id` names the card for the layer's input and
/// blur regions.
///
/// The frame provides its `size` to its content (`widget_scope`), so content that must fit
/// the frame, a `MonthGrid` at `MonthDensity::Auto`, fits itself without being told.
///
/// `shown` plays the card in and out like any surface: it fades in as it mounts shown, and out
/// when the host turns it `Hidden` (a widget the person removes); `on_hidden` runs once the exit
/// has settled, when the host stops drawing it. The host keeps the card until then.
#[component]
pub fn WidgetFrame(
    #[props(default)] size: WidgetSize,
    #[props(default)] host: WidgetHost,
    #[props(default)] tint: CardTint,
    #[props(default)] title: Option<WidgetTitle>,
    #[props(default)] kind: Option<WidgetKind>,
    #[props(default)] lift: Lift,
    #[props(default = Shown::Visible)] shown: Shown,
    #[props(default)] on_hidden: Option<EventHandler<()>>,
    #[props(default)] common: Common,
    children: Element,
) -> Element {
    use_frame_provider(size);
    let Presented { presence, alias } = use_presence(
        shown,
        PresenceSpec {
            enter: Anim::PaletteFade,
            exit: Exit::Fade,
        },
        on_hidden,
    );
    let tint = tint.on(host);
    let kind = kind.map(|kind| kind.as_str().to_owned());
    let class = common.class("ds-widget");
    let data = common.data_attributes();
    let card = rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-size": size.slug(),
            "data-host": host.slug(),
            "data-tint": tint.attr(),
            "data-widget": kind,
            "data-lift": lift.attr(),
            "data-shown": presence.shown().slug(),
            "data-presence": presence.drawn_slug(),
            "data-pulse": alias.slug(),
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            if tint == CardTint::Space {
                div { class: "ds-frame" }
            }
            if let Some(title) = title {
                div { class: "ds-widget-title",
                    Glyph { icon: title.glyph, size: IconSize::Tiny }
                    span { class: "ds-widget-title-text", {text(&title.text)} }
                }
            }
            div { class: "ds-widget-body", {children} }
        }
    };
    match host {
        WidgetHost::Desktop => rsx! {
            Surface { material: Material::Widget, chrome: RootChrome::Transparent, {card} }
        },
        WidgetHost::Tile => card,
    }
}
