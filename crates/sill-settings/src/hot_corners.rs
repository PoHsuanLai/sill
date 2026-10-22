//! `hot_corners.*`: what each screen corner does when the pointer rests in it (a list of
//! bindings, each for the modifier keys held) (design/20 §1.16;
//! macOS "Desktop & Dock" > Hot Corners, hence `Page::Dock`). The section and its keys live in
//! `ds-settings` (`schema/behaviour.rs`), beside the machine's timing in `ds-behaviour`, so the
//! shell and the compositor read one definition; here it is re-exported.

pub use ds_settings::schema::{
    CornerAction, CornerBinding, CornerBindings, CornerDo, CornerKey, CornerKeys,
    HotCornerSettings, OpenItem,
};
