//! Where a surface is being placed: the output, its rank among the outputs, and the settings.

use shell_host::OutputInfo;
use sill_settings::ShellFile;

/// How an output ranks among the daemon's outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputRank {
    /// The first output the compositor announced.
    Primary,
    /// Every other output.
    Secondary,
}

impl OutputRank {
    /// The rank of the output at `index` in the daemon's output order.
    pub fn of_index(index: usize) -> OutputRank {
        match index {
            0 => OutputRank::Primary,
            _ => OutputRank::Secondary,
        }
    }
}

/// What [`super::Surface::slots`] decides from.
#[derive(Debug, Clone, Copy)]
pub struct Place<'a> {
    pub output: &'a OutputInfo,
    pub rank: OutputRank,
    pub settings: &'a ShellFile,
}
