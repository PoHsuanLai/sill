//! The locales a desktop entry's translated words are picked for.

use ds_helpers::EntryLocale;

/// The variable holding the session's language, read first.
const LANG: &str = "LANG";

/// The variable holding further languages, a `:` list, read after [`LANG`].
const LANGUAGES: &str = "LANGUAGES";

/// The session's locales, best first, from this process's environment.
pub fn session_locales() -> Vec<EntryLocale> {
    locales_from(|name| std::env::var(name).ok())
}

/// The locales `var` (an environment lookup) names: `LANG`, then each item of `LANGUAGES`. A value
/// that names no locale (`C`, `POSIX`, empty) adds none.
fn locales_from(var: impl Fn(&str) -> Option<String>) -> Vec<EntryLocale> {
    let languages = var(LANGUAGES).unwrap_or_default();
    var(LANG)
        .into_iter()
        .chain(languages.split(':').map(str::to_owned))
        .filter_map(|text| EntryLocale::parse(&text))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::locales_from;

    #[test]
    fn the_session_locales_are_lang_then_languages() {
        // `(LANG, LANGUAGES, the variants of every locale in order)`.
        const CASES: &[(Option<&str>, Option<&str>, &[&str])] = &[
            (None, None, &[]),
            (Some("C"), None, &[]),
            (Some("C.UTF-8"), Some("POSIX"), &[]),
            (Some("de_AT.UTF-8"), None, &["de_AT", "de"]),
            (Some("sr@latin"), None, &["sr@latin", "sr"]),
            (None, Some("fr:de_AT"), &["fr", "de_AT", "de"]),
            (Some("en_US.UTF-8"), Some("de:"), &["en_US", "en", "de"]),
        ];
        for &(lang, languages, want) in CASES {
            let var = |name: &str| match name {
                "LANG" => lang.map(str::to_owned),
                "LANGUAGES" => languages.map(str::to_owned),
                _ => None,
            };
            let got: Vec<String> = locales_from(var)
                .iter()
                .flat_map(|locale| locale.variants())
                .collect();
            assert_eq!(got, want, "{lang:?} {languages:?}");
        }
    }
}
