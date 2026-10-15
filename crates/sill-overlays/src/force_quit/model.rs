//! The panel's state while it is open, pure: the app selected in the list, and the app the
//! confirmation sheet is asking about.

/// Whether the confirmation sheet is up.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum Confirming {
    /// No question is open; the list takes the keys.
    #[default]
    No,
    /// "Do you want to force quit this app?" is up, about the app with this key.
    App(String),
}

/// One opening's state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Chosen {
    /// The key of the app selected in the list.
    pub app: Option<String>,
    pub confirming: Confirming,
}
