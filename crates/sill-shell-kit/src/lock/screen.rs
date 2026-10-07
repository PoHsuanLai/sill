//! LockScreen: where the lock screen's parts stand (design/20-SURFACES.md section 1.9;
//! design/04-COMPONENTS.md section 42). The wallpaper fills the surface under a light veil, the
//! clock stands near the top and the prompt near the bottom, centred across: the reference lock
//! screen's layout, so a shell draws it without layout rules of its own.

use dioxus::prelude::*;
use ds::components::content::image_source::ImageSource;
use ds::root::common::Common;
use ds::style::icon::url::IconUrl;

/// The lock screen's stage. Put it in a root that fills its surface (`Ds { material:
/// Material::Window, extent: RootExtent::Viewport, .. }`, one per output); `wallpaper` is the
/// output's picture, ideally already blurred by the caller (Blitz blurs nothing, spike S15),
/// drawn to cover the stage under `--lock-veil`. `clock` is a [`crate::lock::clock::LockClock`] and `prompt`
/// a [`crate::lock::prompt::LockPrompt`].
#[component]
pub fn LockScreen(
    #[props(default)] wallpaper: Option<ImageSource>,
    clock: Element,
    prompt: Element,
    #[props(default)] common: Common,
) -> Element {
    let style = wallpaper.and_then(|source| wallpaper_style(&source));
    let class = common.class("ds-lock");
    let data = common.data_attributes();
    let label = common
        .aria_label
        .clone()
        .unwrap_or_else(|| "Locked".to_owned());
    rsx! {
        div {
            class,
            id: common.id.clone(),
            role: "dialog",
            "aria-label": "{label}",
            onmounted: move |event| common.mounted(event),
            ..data,
            div { class: "ds-lock-wallpaper", style, "aria-hidden": "true" }
            div { class: "ds-lock-veil", "aria-hidden": "true" }
            div { class: "ds-lock-top", {clock} }
            div { class: "ds-lock-bottom", {prompt} }
        }
    }
}

/// The wallpaper as an inline `background-image`, its URL escaped for the quoted string; a
/// source that is neither `data:` nor `file:` draws no picture.
fn wallpaper_style(source: &ImageSource) -> Option<String> {
    IconUrl::parse(&source.0)
        .ok()
        .map(|url| format!("background-image:url(\"{}\")", url.as_str()))
}

#[cfg(test)]
mod tests {
    use super::wallpaper_style;
    use ds::components::content::image_source::ImageSource;

    #[test]
    fn only_a_local_picture_is_drawn_and_its_quotes_are_escaped() {
        let local = ImageSource("data:image/png;base64,AA\"A".to_owned());
        assert_eq!(
            wallpaper_style(&local).as_deref(),
            Some("background-image:url(\"data:image/png;base64,AA%22A\")")
        );
        let remote = ImageSource("https://example.com/w.png".to_owned());
        assert_eq!(wallpaper_style(&remote), None);
    }
}
