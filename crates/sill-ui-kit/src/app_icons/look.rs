//! The icon style the dock and the launcher draw in: `icons.style` (quire's
//! appearance file) read against the active Space (Monochrome takes the Space's hue), and the scheme the surface paints
//! (in the dark, Muted and Monochrome lift a raster into quire's tone band, design/29 §11 F3).
//! Pure.

use ds::prelude::*;
use ds::style::icon::retint::IconStyle;
use ds::style::icon::retint::Tint;
use ds_settings::IconsSettings;

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

/// The look `icons` asks for on a Space drawn as `space` in `scheme`.
pub fn icon_look(icons: &IconsSettings, space: &SpaceLook, scheme: Scheme) -> IconLook {
    let style = style_of(icons.style);
    let tint = match style {
        IconStyle::Monochrome => Tint::space(&space.dots),
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

#[cfg(test)]
mod tests {
    use super::{IconLook, icon_look};
    use ds::prelude::*;
    use ds::style::icon::retint::IconStyle;
    use ds::style::icon::retint::Tint;
    use ds::style::space::presets::default_look;
    use ds_settings::IconsSettings;

    fn icons(style: ds_settings::IconStyle) -> IconsSettings {
        let mut icons = IconsSettings::default();
        icons.style = style;
        icons
    }

    fn hue_near(tint: Tint, hue: f32) -> bool {
        ((tint.hue - hue + 540.0).rem_euclid(360.0) - 180.0).abs() < 15.0 && tint.chroma > 0.02
    }

    #[test]
    fn colour_and_muted_ignore_the_tint() {
        use ds_settings::IconStyle as S;
        let work = default_look(0, Grain(35));
        for (style, want) in [(S::Colour, IconStyle::Colour), (S::Muted, IconStyle::Muted)] {
            let look = icon_look(&icons(style), &work, Scheme::Light);
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

    /// Monochrome follows the workspace's preset: Work near 268, Home near 152.
    #[test]
    fn monochrome_takes_the_spaces_hue() {
        use ds_settings::IconStyle::Monochrome;
        let work = default_look(0, Grain(35));
        let home = default_look(1, Grain(55));
        let at = |space| icon_look(&icons(Monochrome), space, Scheme::Light);
        assert!(hue_near(at(&work).tint, 268.0));
        assert!(hue_near(at(&home).tint, 152.0));
        assert_ne!(
            at(&work).tint,
            at(&home).tint,
            "a workspace switch changes the tint"
        );
        assert_eq!(at(&work).style, IconStyle::Monochrome);
    }

    /// Muted and Monochrome keep the surface's scheme (the dark tone band re-keys them);
    /// Colour draws every icon as it comes, so it keys on the light rule whatever the scheme.
    #[test]
    fn only_a_retinted_style_keys_on_the_scheme() {
        use ds_settings::IconStyle as S;
        let work = default_look(0, Grain(35));
        let cases = [
            (S::Colour, Scheme::Light),
            (S::Muted, Scheme::Dark),
            (S::Monochrome, Scheme::Dark),
        ];
        for (style, want) in cases {
            let look = icon_look(&icons(style), &work, Scheme::Dark);
            assert_eq!(look.scheme, want, "{style:?}");
        }
    }
}
