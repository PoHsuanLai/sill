//! The icon style the dock and the launcher draw in: `icons.style` and `icons.monochrome_tint`
//! (quire's appearance file) read against the active Space, and the scheme the surface paints
//! (in the dark, Muted and Monochrome lift a raster into quire's tone band, design/29 §11 F3).
//! Pure.

use ds::prelude::*;
use ds::style::icon::retint::IconStyle;
use ds::style::icon::retint::Tint;
use ds::style::space::look::CardAccent;
use ds::style::space::palette::derive;
use ds::style::tokens::accent_table::accent_of;
use ds_settings::{IconsSettings, MonochromeTint};

/// The style and the tint every app icon on a surface is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconLook {
    pub style: IconStyle,
    /// The Monochrome hue; [`Tint::NEUTRAL`] under the other styles, which ignore it, so a
    /// Space switch never re-keys a Colour or Muted icon.
    pub tint: Tint,
    /// The scheme the raster is re-coloured for; [`Scheme::Light`] under Colour, which draws
    /// every icon as it comes, so a scheme switch never re-keys it.
    pub scheme: Scheme,
}

impl IconLook {
    /// Colour: every icon as it comes.
    pub const COLOUR: IconLook = IconLook {
        style: IconStyle::Colour,
        tint: Tint::NEUTRAL,
        scheme: Scheme::Light,
    };
}

impl Default for IconLook {
    fn default() -> Self {
        IconLook::COLOUR
    }
}

/// The look `icons` asks for on a Space drawn as `space` in `scheme`, with the appearance
/// `accent`.
pub fn icon_look(
    icons: &IconsSettings,
    space: &SpaceLook,
    accent: Accent,
    scheme: Scheme,
) -> IconLook {
    let style = style_of(icons.style);
    let tint = match style {
        IconStyle::Monochrome => tint_of(icons.monochrome_tint, space, accent),
        IconStyle::Colour | IconStyle::Muted => Tint::NEUTRAL,
    };
    let scheme = match style {
        IconStyle::Colour => Scheme::Light,
        IconStyle::Muted | IconStyle::Monochrome => scheme,
    };
    IconLook {
        style,
        tint,
        scheme,
    }
}

/// `icons.style` in `ds::style::icon`'s words.
fn style_of(style: ds_settings::IconStyle) -> IconStyle {
    match style {
        ds_settings::IconStyle::Colour => IconStyle::Colour,
        ds_settings::IconStyle::Muted => IconStyle::Muted,
        ds_settings::IconStyle::Monochrome => IconStyle::Monochrome,
    }
}

/// `icons.monochrome_tint`: the Space's accent, the card accent, or none.
fn tint_of(tint: MonochromeTint, space: &SpaceLook, accent: Accent) -> Tint {
    match tint {
        MonochromeTint::Space => Tint::space(&space.dots),
        MonochromeTint::Accent => {
            Tint::from_hex(&card_accent(space, accent)).unwrap_or(Tint::NEUTRAL)
        }
        MonochromeTint::Neutral => Tint::NEUTRAL,
    }
}

/// The accent a card on this Space shows (light scheme, as `Tint::space` reads it): the Space's
/// own when its card accent follows the Space, else the appearance accent.
fn card_accent(space: &SpaceLook, accent: Accent) -> String {
    match space.card_accent {
        CardAccent::SpaceHue => derive(&space.dots, Scheme::Light).accent,
        CardAccent::Chosen => accent_of(accent, Scheme::Light).fill.css(),
    }
}

#[cfg(test)]
mod tests {
    use super::{IconLook, icon_look};
    use ds::prelude::*;
    use ds::style::icon::retint::IconStyle;
    use ds::style::icon::retint::Tint;
    use ds::style::space::look::CardAccent;
    use ds::style::space::presets::default_look;
    use ds_settings::{IconsSettings, MonochromeTint};

    fn icons(style: ds_settings::IconStyle, tint: MonochromeTint) -> IconsSettings {
        IconsSettings {
            style,
            monochrome_tint: tint,
            ..IconsSettings::default()
        }
    }

    fn hue_near(tint: Tint, hue: f32) -> bool {
        ((tint.hue - hue + 540.0).rem_euclid(360.0) - 180.0).abs() < 15.0 && tint.chroma > 0.02
    }

    #[test]
    fn colour_and_muted_ignore_the_tint() {
        use ds_settings::IconStyle as S;
        let work = default_look(0, CardAccent::Chosen);
        for (style, want) in [(S::Colour, IconStyle::Colour), (S::Muted, IconStyle::Muted)] {
            let look = icon_look(
                &icons(style, MonochromeTint::Space),
                &work,
                Accent::Red,
                Scheme::Light,
            );
            assert_eq!(
                look,
                IconLook {
                    style: want,
                    tint: Tint::NEUTRAL,
                    scheme: Scheme::Light,
                }
            );
        }
    }

    /// Space follows the workspace's preset (Work near 268, Home near 152), Accent the card's
    /// accent (Chosen: the appearance accent; SpaceHue: the Space's), Neutral none.
    #[test]
    fn monochrome_takes_its_tint_from_the_setting() {
        use ds_settings::IconStyle::Monochrome;
        let work = default_look(0, CardAccent::Chosen);
        let home = default_look(1, CardAccent::Chosen);
        let at = |tint, space, accent| {
            icon_look(&icons(Monochrome, tint), space, accent, Scheme::Light).tint
        };
        assert!(hue_near(
            at(MonochromeTint::Space, &work, Accent::Red),
            268.0
        ));
        assert!(hue_near(
            at(MonochromeTint::Space, &home, Accent::Red),
            152.0
        ));
        assert_ne!(
            at(MonochromeTint::Space, &work, Accent::Red),
            at(MonochromeTint::Space, &home, Accent::Red),
            "a workspace switch changes the tint"
        );
        // Red's deep member sits near hue 25-30 in OKLCh.
        assert!(hue_near(
            at(MonochromeTint::Accent, &work, Accent::Red),
            28.0
        ));
        let home_hue = default_look(1, CardAccent::SpaceHue);
        assert!(hue_near(
            at(MonochromeTint::Accent, &home_hue, Accent::Red),
            152.0
        ));
        assert_eq!(
            at(MonochromeTint::Neutral, &work, Accent::Red),
            Tint::NEUTRAL
        );
        assert_eq!(
            icon_look(
                &icons(Monochrome, MonochromeTint::Space),
                &work,
                Accent::Red,
                Scheme::Light,
            )
            .style,
            IconStyle::Monochrome
        );
    }

    /// Muted and Monochrome keep the surface's scheme (the dark tone band re-keys them);
    /// Colour draws every icon as it comes, so it keys on the light rule whatever the scheme.
    #[test]
    fn only_a_retinted_style_keys_on_the_scheme() {
        use ds_settings::IconStyle as S;
        let work = default_look(0, CardAccent::Chosen);
        let cases = [
            (S::Colour, Scheme::Light),
            (S::Muted, Scheme::Dark),
            (S::Monochrome, Scheme::Dark),
        ];
        for (style, want) in cases {
            let look = icon_look(
                &icons(style, MonochromeTint::Space),
                &work,
                Accent::Red,
                Scheme::Dark,
            );
            assert_eq!(look.scheme, want, "{style:?}");
        }
    }
}
