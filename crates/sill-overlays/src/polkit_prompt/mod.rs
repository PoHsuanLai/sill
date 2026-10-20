//! sill's own polkit prompt (design/20 §1.10): one
//! Overlay per output, kept warm and hidden like the power menu, shown on the main display (the
//! banners' `BannerLink` output) while the polkit agent's `current` holds a request: quire's
//! `PolkitPrompt` (a narrow `Sheet` over the modal scrim, `peek-in` in and `sheet-out` out: the
//! avatar and name of the identity asked for, the action's sentence, the details as a hover
//! card, a secret field, Cancel and Authenticate; `Wrong` shakes once) in `Ds { material: Sheet,
//! extent: Viewport }` (`view.rs`). Keyboard `Exclusive`; Enter submits and Escape
//! and the scrim cancel, all inside quire's component.
//!
//! An answer goes to the agent as `PolkitCommand::Answer` (the text wrapped in a `Secret` at
//! once, never kept), a cancel as `Dismiss`; the surface never talks to polkit.
//!
//! The daemon tells the service to register at start; it stays passive until then. Registering an
//! agent is per logind session: nested in another desktop's session, the daemon shares that
//! session, whose own agent already holds it, so polkitd refuses sill's
//! (`Registration::Refused`), and that is the right outcome there. Acceptance runs the agent on
//! a private bus with a stand-in Authority, never on the system bus.

mod model;
mod surface;
mod view;

pub use surface::PolkitPrompt;
