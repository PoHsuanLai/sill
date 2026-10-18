//! Clap's parser for a closed vocabulary: the words of a [`Word`] enum, so a command line takes
//! the wire enum itself and no mirror enum maps onto it.

use clap::builder::{PossibleValuesParser, TypedValueParser};
use ds_core::word::Word;

/// A value that is one of `T`'s slugs, parsed to `T`.
pub(super) fn word<T: Word + Send + Sync>() -> impl TypedValueParser<Value = T> {
    PossibleValuesParser::new(T::ALL.iter().map(|word| word.slug()))
        .try_map(|slug| T::parse(&slug).ok_or("not one of the words"))
}
