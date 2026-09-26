//! Which lock surface shows the password prompt: one shared channel the daemon writes (at lock:
//! the main display, the banners' output; then whichever lock surface gains keyboard focus) and
//! every lock surface follows. Implemented at the freeze: the daemon's wiring depends on it.

use shell_host::OutputId;
use tokio::sync::watch;

/// Where the prompt is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PromptOutput {
    #[default]
    Nowhere,
    On(OutputId),
}

/// The daemon's half and every lock surface's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct LockLink {
    prompt: watch::Sender<PromptOutput>,
}

impl Default for LockLink {
    fn default() -> Self {
        LockLink::new()
    }
}

impl LockLink {
    pub fn new() -> LockLink {
        LockLink {
            prompt: watch::Sender::new(PromptOutput::Nowhere),
        }
    }

    /// Show the prompt on `output` (a repeat notifies nobody).
    pub fn set(&self, output: PromptOutput) {
        self.prompt.send_if_modified(|now| {
            let changed = *now != output;
            *now = output;
            changed
        });
    }

    pub fn now(&self) -> PromptOutput {
        *self.prompt.borrow()
    }

    pub fn subscribe(&self) -> watch::Receiver<PromptOutput> {
        self.prompt.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{LockLink, PromptOutput};

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
