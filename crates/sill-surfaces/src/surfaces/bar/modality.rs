//! The bar popup's input modality, which quire's focus ring follows: `Ds` stamps
//! `data-modality` from a `ds::HostModality` root context, and the ring is
//! `.ds[data-modality=keyboard] :focus` (Blitz never matches `:focus-visible`, quire spike S12).
//! `ds_native::launch` provides that context from the raw input it sees; shell-host's surfaces
//! get none (FINDINGS G390), so a Tab through the control center moved the focus with no ring to
//! show it (F992). The popup's frame provides it and reports each key and press that reaches it.

use dioxus::prelude::*;
use ds::{HostModality, InputModality};

/// The last input's kind, as the popup root's `HostModality`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupModality(Signal<InputModality>);

impl PopupModality {
    fn set(self, next: InputModality) {
        let PopupModality(mut current) = self;
        if *current.peek() != next {
            current.set(next);
        }
    }
}

/// The popup's frame: a `div` of `class="sill-popup-frame"` with `style`, around the popup's
/// `Ds` (its child), which reads the modality it provides: pointer at first (the popup opens
/// from a click, or from `sill control-center toggle`), keyboard from the first key that
/// reaches the frame (the ring shows), pointer again from the next press (it hides).
#[component]
pub(crate) fn ModalityFrame(style: String, children: Element) -> Element {
    let HostModality(signal) =
        use_context_provider(|| HostModality(Signal::new(InputModality::Pointer)));
    let modality = PopupModality(signal);
    rsx! {
        div {
            class: "sill-popup-frame",
            style,
            onkeydown: move |_| modality.set(InputModality::Keyboard),
            onpointerdown: move |_| modality.set(InputModality::Pointer),
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use dioxus::prelude::*;
    use ds::{Appearance, Ds, Key, Material};
    use ds_native::{Clock, Harness, HarnessConfig, Viewport};

    use super::ModalityFrame;

    fn app() -> Element {
        rsx! {
            ModalityFrame { style: "width:200px;height:100px".to_owned(),
                Ds { appearance: Appearance::default(), material: Material::Popover,
                    div {
                        class: "panel",
                        tabindex: "-1",
                        onmounted: move |event: MountedEvent| ds::focus_soon(event.data()),
                        button { class: "one", "One" }
                        button { class: "two", "Two" }
                    }
                }
            }
        }
    }

    #[test]
    fn a_key_turns_the_ring_on_and_a_press_turns_it_off() {
        let viewport = Viewport {
            width: 200,
            height: 100,
            scale_percent: 100,
        };
        let mut harness =
            Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
        harness.advance(Duration::from_millis(100));
        let modality = |harness: &Harness| harness.attr(".ds", "data-modality");
        assert_eq!(modality(&harness).as_deref(), Some("pointer"), "at first");
        harness.key(Key::Tab);
        harness.advance(Duration::from_millis(50));
        assert!(harness.is_focused(".one"), "Tab walks from the panel");
        assert_eq!(modality(&harness).as_deref(), Some("keyboard"), "after Tab");
        let two = harness
            .centre(".two")
            .expect("the second button is laid out"); // test-only
        harness.click(two);
        harness.advance(Duration::from_millis(50));
        assert_eq!(
            modality(&harness).as_deref(),
            Some("pointer"),
            "after a press"
        );
    }
}
