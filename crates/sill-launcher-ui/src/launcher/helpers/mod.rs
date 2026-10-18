//! A helper's card in the companion's preview pane: its name, what it is for, the apps it may
//! use and how far, its model, its recent jobs, and the buttons to pause, resume or remove it,
//! with a field to give it a job; for a helper that runs an app, the sign-in and the terminal
//! buttons (Open in Terminal, In Terminal with Bring Back); and its routines.
//! The card is the pane of the selected helper row (or of the "New Helper" row); every change goes to the companion service as a [`HelpersMsg`] and comes
//! back through its state, so this draws what the model says and decides nothing.
//!
//! [`HelpersMsg`]: sill_model::companion::HelpersMsg

mod agent;
mod automatic;
mod card;
mod form;
mod handover;
mod jobs;
mod proposals;
mod routines;
mod say;

pub(crate) use card::HelperCardPane;
