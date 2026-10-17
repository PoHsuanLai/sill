//! The picture the lock screen shows for the session's person: AccountsService's `IconFile`
//! (`org.freedesktop.Accounts`, the system bus, read by the live side) else `$HOME/.face`. Pure
//! here; the reads are the live side's. Who the person is comes from [`crate::env::Passwd`].

use std::path::{Path, PathBuf};

use sill_model::auth::AvatarSource;

/// The picture: AccountsService's icon when it names an existing file (`exists`), else
/// `home/.face` when that exists, else initials.
pub(crate) fn avatar_source(
    accounts_icon: Option<PathBuf>,
    home: &Path,
    exists: impl Fn(&Path) -> bool,
) -> AvatarSource {
    let face = home.join(".face");
    accounts_icon
        .into_iter()
        .chain(std::iter::once(face))
        .find(|path| exists(path))
        .map_or(AvatarSource::Initials, AvatarSource::File)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{avatar_source, user_name_in_passwd};
    use sill_model::auth::AvatarSource;

    #[test]
    fn the_picture_prefers_accountsservice() {
        let icon = PathBuf::from("/var/lib/AccountsService/icons/ada");
        let face = PathBuf::from("/home/ada/.face");
        let home = Path::new("/home/ada");
        let cases: Vec<(Option<PathBuf>, Vec<&PathBuf>, AvatarSource)> = vec![
            (
                Some(icon.clone()),
                vec![&icon, &face],
                AvatarSource::File(icon.clone()),
            ),
            (
                Some(icon.clone()),
                vec![&face],
                AvatarSource::File(face.clone()),
            ),
            (None, vec![&face], AvatarSource::File(face.clone())),
            (None, vec![], AvatarSource::Initials),
        ];
        for (accounts, present, want) in cases {
            let got = avatar_source(accounts.clone(), home, |p| present.iter().any(|q| *q == p));
            assert_eq!(got, want, "{accounts:?} {present:?}");
        }
    }
}
