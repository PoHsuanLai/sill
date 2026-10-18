//! Which lock surface shows the password prompt: one shared channel the daemon writes (at lock:
//! the main display, the banners' output; then whichever lock surface gains keyboard focus) and
//! every lock surface follows.

use shell_host::OutputId;
use tokio::sync::watch;

use super::Shared;

/// Where the prompt is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PromptOutput {
    #[default]
    Nowhere,
    On(OutputId),
}

/// The daemon's half and every lock surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct LockLink {
    prompt: Shared<PromptOutput>,
}

impl LockLink {
    pub fn new() -> LockLink {
        LockLink::default()
    }

    /// Show the prompt on `output` (a repeat notifies nobody).
    pub fn set(&self, output: PromptOutput) {
        self.prompt.set(output);
    }

    pub fn now(&self) -> PromptOutput {
        self.prompt.now()
    }

    pub fn subscribe(&self) -> watch::Receiver<PromptOutput> {
        self.prompt.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use crate::links::{LockLink, PromptOutput};

    #[test]
    fn the_prompt_moves_and_a_repeat_is_silent() {
        let link = LockLink::new();
        let mut seen = link.subscribe();
        assert_eq!(link.now(), PromptOutput::Nowhere);
        link.set(PromptOutput::On(OutputId(2)));
        assert_eq!(seen.has_changed().ok(), Some(true));
        seen.mark_unchanged();
        link.set(PromptOutput::On(OutputId(2)));
        assert_eq!(seen.has_changed().ok(), Some(false));
        assert_eq!(link.now(), PromptOutput::On(OutputId(2)));
    }
}
