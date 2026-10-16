//! Every word of the compositor's permission question, in one file (wave 2 design 3.7): the
//! headline for each power, the line for a program the desktop cannot recognise, the line every
//! question ends with, and the buttons. The sentence a power's body opens with is the shared
//! vocabulary's ([`Power::blurb`]), so the sheet and the Privacy page in Settings say the same
//! thing. Plain words, no protocol names.

use casement_protocols::vocab::{Memory, Power};

/// The yes button.
pub(super) const ALLOW: &str = "Allow";
/// The yes button for a program that is asked again next time.
pub(super) const ALLOW_ONCE: &str = "Allow Once";
/// The no button.
pub(super) const DONT_ALLOW: &str = "Don\u{2019}t Allow";

/// What a program the shell can give no name is called.
pub(super) const UNNAMED_PROGRAM: &str = "A program";

/// Added after the power's sentence when the answer is not kept.
const THIS_TIME_ONLY_LINE: &str =
    "This program can\u{2019}t be recognised, so you will be asked again next time.";

/// The last line of every question.
const SETTINGS_LINE: &str = "You can change this later in Settings, under Privacy.";

/// What `power` lets a program do, as the end of the headline.
const fn wish(power: Power) -> &'static str {
    match power {
        Power::Screen => "would like to see your screen.",
        Power::Keys => "would like to type and click for you.",
        Power::TypingHelp => "would like to help you type.",
        Power::Clipboard => "would like to see and change what you copy.",
        Power::Lock => "would like to lock your screen.",
        Power::Displays => "would like to change your displays.",
        Power::Windows => "would like to see and manage your windows.",
    }
}

/// The headline for a program called `name`.
pub(super) fn headline(power: Power, name: &str) -> String {
    format!("\u{201c}{name}\u{201d} {}", wish(power))
}

/// The explanation: what the power allows, the extra line when the answer is not kept, and the
/// line about Settings.
pub(super) fn body(power: Power, memory: Memory) -> String {
    let extra = match memory {
        Memory::ThisTimeOnly => Some(THIS_TIME_ONLY_LINE),
        Memory::Remembered => None,
    };
    std::iter::once(power.blurb())
        .chain(extra)
        .chain(std::iter::once(SETTINGS_LINE))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The yes button's words: "Allow Once" when nothing is kept.
pub(super) const fn grant_words(memory: Memory) -> &'static str {
    match memory {
        Memory::Remembered => ALLOW,
        Memory::ThisTimeOnly => ALLOW_ONCE,
    }
}
