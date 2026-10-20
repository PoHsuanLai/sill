//! The input source service's state: how the last switch went.

use serde::{Deserialize, Serialize};

/// An input-method framework that may run on the session bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputFramework {
    Fcitx5,
    IBus,
}

/// Why a switch did not happen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum InputRefusal {
    /// No input-method framework is on the bus, so there are no input sources to switch.
    NoFramework,
    /// A framework is running that the shell cannot switch yet.
    Unsupported(InputFramework),
    /// The framework has no input source to switch to.
    NothingToSwitchTo,
    /// The framework did not take the call; the message is its error.
    Failed(String),
}

/// How the last Next Input Source went.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum InputSwitch {
    /// Nothing has been asked yet.
    #[default]
    Idle,
    /// The framework was told to use this input method.
    Switched(String),
    Refused(InputRefusal),
}
