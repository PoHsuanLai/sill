//! The state types: casement's night light as the shell reads and writes it, the manual
//! turn-on, and whether the shell can reach casement's file.

use ds_core::word::Word;

/// When casement warms the screen (`night_light.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
#[word(case = snake)]
pub enum Schedule {
    /// Never.
    #[default]
    Off,
    /// All day.
    Always,
    /// From `night_light.from` to `night_light.to`.
    Scheduled,
}

/// A time of day in whole minutes after midnight, always below 1440.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinuteOfDay(u16);

impl MinuteOfDay {
    /// `minutes` after midnight, wrapped round the day.
    pub fn wrapping(minutes: u32) -> MinuteOfDay {
        MinuteOfDay(u16::try_from(minutes % 1440).unwrap_or(0))
    }

    /// The minutes after midnight.
    pub fn minutes(self) -> u16 {
        self.0
    }
}

/// The colour temperature the screen warms to, in kelvin: lower is warmer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Warmth(u16);

impl Warmth {
    /// The warmest casement takes.
    pub const WARMEST: u16 = 1000;
    /// A neutral screen.
    pub const NEUTRAL: u16 = 6500;
    /// casement's default for the night.
    pub const NIGHT: Warmth = Warmth(3400);

    /// `kelvin`, kept between the warmest and neutral.
    pub fn clamped(kelvin: u32) -> Warmth {
        let kept = kelvin.clamp(u32::from(Warmth::WARMEST), u32::from(Warmth::NEUTRAL));
        Warmth(u16::try_from(kept).unwrap_or(Warmth::NEUTRAL))
    }

    /// The temperature in kelvin.
    pub fn kelvin(self) -> u16 {
        self.0
    }
}

/// casement's `[night_light]` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NightLight {
    pub schedule: Schedule,
    pub from: MinuteOfDay,
    pub to: MinuteOfDay,
    pub warmth: Warmth,
    /// How long the screen takes to warm up and to cool down.
    pub fade_minutes: u16,
}

impl Default for NightLight {
    /// casement's defaults: off, 22:00 to 07:00, 3400 K, a half-hour fade.
    fn default() -> Self {
        NightLight {
            schedule: Schedule::Off,
            from: MinuteOfDay::wrapping(22 * 60),
            to: MinuteOfDay::wrapping(7 * 60),
            warmth: Warmth::NIGHT,
            fade_minutes: 30,
        }
    }
}

/// Seconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnixSeconds(pub i64);

/// "Turn On Until Tomorrow": casement is set to warm all day until `until`, then put back as
/// `restore` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manual {
    pub until: UnixSeconds,
    pub restore: NightLight,
}

/// Who warms the monitors that answer DDC/CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// casement, in software, like every other display.
    #[default]
    Software,
    /// sill, over each monitor's own colour gains (`display.night_warmth`); casement is told to
    /// leave those monitors alone (`night_light.skip_outputs`) and keeps warming the rest.
    Hardware,
}

/// Whether the shell has read casement's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Reach {
    /// Not read yet, or the file could not be used.
    #[default]
    Unavailable,
    Ready,
}

/// What the shell knows of Night Shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NightShift {
    pub reach: Reach,
    pub light: NightLight,
    pub manual: Option<Manual>,
    pub backend: Backend,
}
