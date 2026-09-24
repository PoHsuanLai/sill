//! cctk and sctk handler impls for [`super::state::WlState`]: each turns a protocol callback
//! into a [`super::ToplevelEvent`] or a [`super::WorkspaceEvent`] and publishes the reduced
//! state after `done`.

#![allow(unused_variables, dead_code)] // Freeze stubs

// M1 svc-cosmic: impl ToplevelInfoHandler, ToplevelManagerHandler, WorkspaceHandler,
// SeatHandler, ProvidesRegistryState for WlState, and the delegate_* macros.
