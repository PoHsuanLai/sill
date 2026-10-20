//! Which input method comes after the current one.

/// The entry after `current` in `list`, wrapping round to the first. When `current` is not in the
/// list (a group was edited under it) the first entry; `None` for an empty list. A list of one
/// gives that one.
pub fn next_of<'a>(current: &str, list: &'a [String]) -> Option<&'a str> {
    let at = list.iter().position(|name| name == current);
    let after = at.map_or(0, |at| (at + 1) % list.len());
    list.get(after).map(String::as_str)
}
