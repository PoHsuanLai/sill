//! `use_surface_tokens`: a surface's geometry tokens as the cascade resolves them (quire's
//! `ds_blitz::use_tokens`), read again on the frame after the person's styles or the look change.
//!
//! The document is mutably borrowed while a component renders, so a token read in render is
//! always `None`. The hook reads in a task instead: one frame after the element mounts and one
//! frame after each `Styles` change (the document restyles on the frame after the `<style>` text
//! changes), it runs the surface's reader and keeps the result in a signal, which the component
//! reads in render. Until the first read lands the value is the reader's with nothing resolved,
//! the shipped one.

use std::time::Duration;

use dioxus::prelude::*;
use ds::base::time::clock::sleep;
use ds::prelude::Px;
use ds::style::kit::Styles;
use ds_blitz::{Tokens, use_tokens};

/// A little over one frame at 60 Hz: the document has restyled by then.
const SETTLE: Duration = Duration::from_millis(34);

/// A surface's reader: its tokens from the resolved length of each custom property it names.
pub type ReadTokens<T> = fn(&dyn Fn(&str) -> Option<Px>) -> T;

/// The tokens one element of a surface sees, as its reader makes them.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceTokens<T: 'static> {
    tokens: Tokens,
    read: ReadTokens<T>,
    value: Signal<T>,
}

/// The tokens for the calling component, read by `read` (`|length| XTokens::read(length)`) and
/// read again after `styles` (`RootStyle::styles`) changes. Give [`SurfaceTokens::mounted`] to
/// the `onmounted` of an element inside the surface's `Ds` root.
pub fn use_surface_tokens<T: Copy + PartialEq + 'static>(
    styles: ReadSignal<Styles>,
    read: ReadTokens<T>,
) -> SurfaceTokens<T> {
    let tokens = use_tokens();
    let value = use_signal(|| read(&|_| None));
    let surface = SurfaceTokens {
        tokens,
        read,
        value,
    };
    use_effect(move || {
        let _changed = styles.read();
        surface.settle();
    });
    surface
}

impl<T: Copy + PartialEq + 'static> SurfaceTokens<T> {
    /// Remember the element whose tokens this reads, and read them once it is styled.
    pub fn mounted(self, event: MountedEvent) {
        self.tokens.mounted(event);
        self.settle();
    }

    /// The tokens now: the shipped values until the first read lands.
    pub fn get(&self) -> T {
        *self.value.read()
    }

    /// Read the tokens after the document has had a frame, outside render.
    fn settle(self) {
        let mut value = self.value;
        spawn(async move {
            sleep(SETTLE).await;
            let read = (self.read)(&|name| self.tokens.length(name));
            if *value.peek() != read {
                value.set(read);
            }
        });
    }
}
