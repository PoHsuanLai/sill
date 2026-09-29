//! Which of the wallpaper's own two image layers is in front: the same A/B swap `Ds` uses for
//! its frame gradient (quire `root/ds.rs`'s `FrameLayers`), reimplemented here rather than
//! shared because the wallpaper is not a `Material::Window` frame (design/21 §8: it does not
//! tint, and is independent of the Space) — it cross-fades a `background-image` URL, not a
//! gradient. Pure: [`Layers::show`] decides which DOM node gets the new image and which one
//! fades out, with no host, no signal, no timer.

/// Which slot is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Front {
    A,
    B,
}

/// The two layers' current image — `None` shows nothing, so `.sill-wallpaper`'s own
/// `--paper` background shows through — and which is in front.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layers {
    a: Option<String>,
    b: Option<String>,
    front: Front,
}

impl Layers {
    /// Both layers start on `image`, so the very first render never cross-fades from nothing.
    pub fn starting_at(image: Option<String>) -> Layers {
        Layers {
            a: image.clone(),
            b: image,
            front: Front::A,
        }
    }

    /// Show `image`: when it differs from what is already showing, the hidden layer takes it
    /// and comes to the front, so the stylesheet's opacity transition (`--t-big`) cross-fades
    /// the two. A no-op when `image` is already showing — "renders only on change" (design/20
    /// §1.4): no layer is touched, so the component's markup is identical and Dioxus emits no
    /// patch.
    pub fn show(self, image: Option<String>) -> Layers {
        let showing = match self.front {
            Front::A => &self.a,
            Front::B => &self.b,
        };
        if *showing == image {
            return self;
        }
        match self.front {
            Front::A => Layers {
                b: image,
                front: Front::B,
                ..self
            },
            Front::B => Layers {
                a: image,
                front: Front::A,
                ..self
            },
        }
    }

    /// Each layer's key, its `data-layer` attribute (`Some("back")` for the hidden one, `None`
    /// for the front — the stylesheet's `[*|data-layer=back]{opacity:0}` rule), and its image,
    /// in a fixed order so the two DOM nodes persist across renders.
    pub fn render(&self) -> [(&'static str, Option<&'static str>, Option<String>); 2] {
        let data_layer = |slot| {
            if slot == self.front {
                None
            } else {
                Some("back")
            }
        };
        [
            ("a", data_layer(Front::A), self.a.clone()),
            ("b", data_layer(Front::B), self.b.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Layers;

    fn image(name: &str) -> Option<String> {
        Some(format!("file:///{name}.png"))
    }

    #[test]
    fn starting_at_shows_the_same_image_on_both_layers_with_a_in_front() {
        let layers = Layers::starting_at(image("sea"));
        let rendered = layers.render();
        assert_eq!(rendered[0], ("a", None, image("sea")));
        assert_eq!(rendered[1], ("b", Some("back"), image("sea")));
    }

    #[test]
    fn starting_at_none_shows_neither_layer() {
        let layers = Layers::starting_at(None);
        for (_, _, shown) in layers.render() {
            assert_eq!(shown, None);
        }
    }

    #[test]
    fn showing_the_same_image_again_changes_nothing() {
        let layers = Layers::starting_at(image("sea"));
        let again = layers.clone().show(image("sea"));
        assert_eq!(layers, again);
    }

    #[test]
    fn a_new_image_goes_on_the_hidden_layer_and_becomes_the_front() {
        let layers = Layers::starting_at(image("sea")).show(image("mountain"));
        let rendered = layers.render();
        // "a" was in front showing "sea"; the new image goes on "b", which then fronts.
        assert_eq!(rendered[0], ("a", Some("back"), image("sea")));
        assert_eq!(rendered[1], ("b", None, image("mountain")));
    }

    #[test]
    fn the_front_alternates_every_change_so_both_layers_keep_their_own_node() {
        let layers = Layers::starting_at(image("one"))
            .show(image("two"))
            .show(image("three"));
        let rendered = layers.render();
        // "one" -> "two" put "two" on "b" (front); "two" -> "three" then puts "three" on "a"
        // (front), leaving "two" behind on "b" — each change alternates which node changes.
        assert_eq!(rendered[0], ("a", None, image("three")));
        assert_eq!(rendered[1], ("b", Some("back"), image("two")));
    }

    #[test]
    fn a_scheme_change_to_no_image_fades_out_to_paper() {
        let layers = Layers::starting_at(image("sea")).show(None);
        let rendered = layers.render();
        assert_eq!(rendered[1], ("b", None, None));
        assert_eq!(rendered[0], ("a", Some("back"), image("sea")));
    }
}
