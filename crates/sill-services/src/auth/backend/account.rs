//! Who the lock screen asks for: the session's user from `/etc/passwd` (by this process's uid),
//! and the picture, AccountsService's `IconFile` (`org.freedesktop.Accounts`, the system bus,
//! read by the live side) else `$HOME/.face`. Pure here; the reads are the live side's.

use std::path::{Path, PathBuf};

use sill_model::auth::{AvatarSource, RealName, UserName};

/// The login name and the GECOS name (its first comma-separated field; the login name when
/// empty) of `uid` in `passwd` (the file's text).
pub(crate) fn user_name_in_passwd(passwd: &str, uid: u32) -> Option<(UserName, RealName)> {
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        let user = fields.next()?;
        let _password = fields.next()?;
        let line_uid: u32 = fields.next()?.parse().ok()?;
        let _gid = fields.next()?;
        let gecos = fields.next()?;
        (line_uid == uid && !user.is_empty()).then(|| {
            let name = gecos.split(',').next().unwrap_or_default().trim();
            let name = if name.is_empty() { user } else { name };
            (UserName(user.to_owned()), RealName(name.to_owned()))
        })
    })
}

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
    use sill_model::auth::{AvatarSource, RealName, UserName};

    const PASSWD: &str = "root:x:0:0:Super User:/root:/bin/bash\n\
        ada:x:1000:1000:Ada Lovelace,,,:/home/ada:/bin/zsh\n\
        bob:x:1001:1001::/home/bob:/bin/sh\n\
        # a comment\n\
        broken line\n";

    #[test]
    fn passwd_lines() {
        let named =
            |user: &str, name: &str| Some((UserName(user.to_owned()), RealName(name.to_owned())));
        let cases = [
            (1000, named("ada", "Ada Lovelace")),
            (1001, named("bob", "bob")),
            (0, named("root", "Super User")),
            (4242, None),
        ];
        for (uid, want) in cases {
            assert_eq!(user_name_in_passwd(PASSWD, uid), want, "{uid}");
        }
    }

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
