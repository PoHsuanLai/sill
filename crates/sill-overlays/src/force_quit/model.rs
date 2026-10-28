//! The panel's state while it is open, pure: the app selected in the list, and the app the
//! confirmation sheet is asking about.

/// One opening's state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Chosen {
    /// The key of the app selected in the list.
    pub app: Option<String>,
    /// The key of the app "Do you want to force quit this app?" is up about; with no question
    /// open (`None`) the list takes the keys.
    pub confirming: Option<String>,
}
