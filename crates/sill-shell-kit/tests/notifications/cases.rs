//! The notification parts' specimens, each in a Toast root of its scheme.

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::components::content::rich_text::{Rich, RichRun, RichText};
use ds::components::content::text_runs::RunTone;
use ds::prelude::*;
use ds::root::common::Common;
use sill_shell_kit::notifications::banner_stack::{Banner, BannerKey, BannerPosition};
use sill_shell_kit::notifications::parts::{AppMark, CardAction, GroupCount, StackLayers};
use sill_shell_kit::prelude::*;

/// A specimen: its golden name and how it is made.
pub type Specimen = (&'static str, fn() -> Element);

/// Every specimen.
pub const SPECIMENS: &[Specimen] = &[
    ("rich-runs", rich_runs),
    ("group-header-closed", group_header_closed),
    ("group-header-open-dark", group_header_open_dark),
    ("card-light", card_light),
    ("card-dark", card_dark),
    ("card-group", card_group),
    ("card-popover-root", card_popover_root),
    ("stack-top-right", stack_top_right),
    ("stack-bottom-right-dark", stack_bottom_right_dark),
    ("center-light", center_light),
    ("center-dark", center_dark),
];

/// `body` in a Toast root in `theme`.
fn toast(theme: Theme, body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, ..Appearance::default() }, material: Material::Toast, stylesheet: Inject::Host,
            {body}
        }
    }
}

/// A body with every tone and a link.
pub fn rich_runs() -> Element {
    let body = Rich(vec![
        RichRun::Run(TextRun::new("Build ", RunTone::Plain)),
        RichRun::Run(TextRun::new("#42", RunTone::Strong)),
        RichRun::Run(TextRun::new(" passed ", RunTone::Plain)),
        RichRun::Run(TextRun::new("again", RunTone::Italic)),
        RichRun::Run(TextRun::new(", ", RunTone::Plain)),
        RichRun::Run(TextRun::new("no flakes", RunTone::Underline)),
        RichRun::Run(TextRun::new(". See ", RunTone::Faint)),
        RichRun::link("the run", "https://ci.example/runs/42"),
        RichRun::Run(TextRun::new(".", RunTone::Plain)),
    ]);
    toast(
        Theme::Light,
        rsx! { p { RichText { body, on_link: |_| {} } } },
    )
}

/// A folded group of four from Mail.
pub fn group_header_closed() -> Element {
    toast(
        Theme::Light,
        rsx! {
            GroupHeader { icon: IconSource::Glyph(Icon::Mail), name: "Mail", count: 4, on_toggle: |_| {}, on_clear: |_| {} }
        },
    )
}

/// An open group, in the dark scheme.
pub fn group_header_open_dark() -> Element {
    toast(
        Theme::Dark,
        rsx! {
            GroupHeader { icon: IconSource::Glyph(Icon::Terminal), name: "Terminal", count: 2, expanded: Shown::Visible, on_toggle: |_| {}, on_clear: |_| {} }
        },
    )
}

/// The Mail app's mark.
fn mail() -> AppMark {
    AppMark {
        icon: IconSource::Glyph(Icon::Mail),
        name: "Mail".into(),
    }
}

/// A plain card with a body that runs past two lines.
fn card(theme: Theme) -> Element {
    toast(
        theme,
        rsx! {
            NotificationCard {
                app: mail(),
                age: "now",
                summary: "Ada Lovelace",
                body: "I have translated the memoir and added notes of my own, which run rather longer than the memoir itself.",
                on_close: |_| {},
                on_open: |_| {},
            }
        },
    )
}

/// Light.
pub fn card_light() -> Element {
    card(Theme::Light)
}

/// Dark.
pub fn card_dark() -> Element {
    card(Theme::Dark)
}

/// A group of three with two layers behind it, two actions and a link in the body.
pub fn card_group() -> Element {
    let body = Rich(vec![
        RichRun::Run(TextRun::new("Build ", RunTone::Plain)),
        RichRun::Run(TextRun::new("#42", RunTone::Strong)),
        RichRun::Run(TextRun::new(" passed. ", RunTone::Plain)),
        RichRun::link("Open the run", "https://ci.example/runs/42"),
    ]);
    toast(
        Theme::Light,
        rsx! {
            NotificationCard {
                app: AppMark { icon: IconSource::Glyph(Icon::Terminal), name: "CI".into() },
                age: "2m",
                summary: "main is green",
                body,
                count: GroupCount { count: 3, layers: StackLayers(2) },
                actions: vec![
                    CardAction { label: "Rerun".into(), on_press: EventHandler::new(|_| {}) },
                    CardAction { label: "Mute".into(), on_press: EventHandler::new(|_| {}) },
                ],
                on_close: |_| {},
                on_open: |_| {},
                on_link: |_| {},
                common: Common { id: Some("toast-7".to_string()), ..Common::default() },
            }
        },
    )
}

/// A Toast card inside a Popover root: its own transparent scope paints the Toast material.
pub fn card_popover_root() -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host,
            NotificationCard { app: mail(), age: "9:41", summary: "Grace Hopper", on_close: |_| {}, on_open: |_| {} }
        }
    }
}

/// Two banners in a stack at `position` in `theme`.
fn stack(theme: Theme, position: BannerPosition) -> Element {
    let banners = [(2, "Grace Hopper", "9:41"), (1, "Ada Lovelace", "now")]
        .into_iter()
        .map(|(n, who, age)| Banner {
            key: BannerKey(n),
            card: rsx! {
                NotificationCard { app: mail(), age, summary: who, body: "Are we still on for Thursday?", on_close: |_| {}, on_open: |_| {} }
            },
        })
        .collect::<Vec<_>>();
    toast(theme, rsx! { BannerStack { banners, position } })
}

/// Top right, light.
pub fn stack_top_right() -> Element {
    stack(Theme::Light, BannerPosition::TopRight)
}

/// Bottom right, dark.
pub fn stack_bottom_right_dark() -> Element {
    stack(Theme::Dark, BannerPosition::BottomRight)
}

/// The center in `theme`: a group header over two cards, in a Popover root.
fn center(theme: Theme) -> Element {
    rsx! {
        Ds { appearance: Appearance { theme, ..Appearance::default() }, material: Material::Popover, stylesheet: Inject::Host, extent: RootExtent::Viewport,
            SidePanel { label: "Notification Center", shown: Shown::Visible, width: Px(384.0), onclose: |_| {},
                GroupHeader { icon: IconSource::Glyph(Icon::Mail), name: "Mail", count: 2, expanded: Shown::Visible, on_toggle: |_| {}, on_clear: |_| {} }
                NotificationCard { app: mail(), age: "9:41", summary: "Grace Hopper", body: "Are we still on for Thursday?", on_close: |_| {}, on_open: |_| {} }
                NotificationCard { app: mail(), age: "9:12", summary: "Ada Lovelace", on_close: |_| {}, on_open: |_| {} }
            }
        }
    }
}

/// Light.
pub fn center_light() -> Element {
    center(Theme::Light)
}

/// Dark.
pub fn center_dark() -> Element {
    center(Theme::Dark)
}
