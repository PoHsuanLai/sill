//! How much of the media players the shell holds. A player is another program, and everything it
//! says is its own choice, so each part is bounded where it enters.

/// How many players are listed: a desktop has a handful of media apps, and a program that claims
/// hundreds of player names is not asking for them to be seen.
pub const PLAYERS_MAX: usize = 16;

/// The characters of a title, an artist, an album or a player's name that are kept: a card shows
/// one line of each.
pub const TEXT_MAX_CHARS: usize = 200;

/// How many artists of a track are kept.
pub const ARTISTS_MAX: usize = 8;
