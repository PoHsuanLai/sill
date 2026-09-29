//! Which of polkitd's identities to ask for: the session's own user when listed, else root, else
//! the first `unix-user`; groups and other kinds are never chosen (polkitd lists a
//! group's members as users when it wants them).

use crate::auth::UserName;

use super::types::{Identity, Uid};

/// The uid to authenticate as, `own` being this session's user.
pub fn pick_identity(identities: &[Identity], own: Uid) -> Option<Uid> {
    let users: Vec<Uid> = identities
        .iter()
        .filter_map(|identity| match identity {
            Identity::UnixUser(uid) => Some(*uid),
            Identity::UnixGroup(_) | Identity::Other(_) => None,
        })
        .collect();
    [own, Uid(0)]
        .into_iter()
        .find(|wanted| users.contains(wanted))
        .or_else(|| users.first().copied())
}

/// The login name of `uid` in `passwd` (the file's text): the first field of the line whose
/// third field is `uid`. Comments, malformed lines and empty names are skipped.
pub fn name_in_passwd(passwd: &str, uid: Uid) -> Option<UserName> {
    passwd
        .lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let id = fields.nth(1)?.parse::<u32>().ok()?;
            (id == uid.0 && !name.is_empty()).then(|| UserName(name.to_owned()))
        })
}

#[cfg(test)]
mod tests {
    use super::{name_in_passwd, pick_identity};
    use crate::auth::UserName;
    use crate::polkit_agent::types::{Gid, Identity, Uid};

    #[test]
    fn own_then_root_then_first_user() {
        let user = |uid| Identity::UnixUser(Uid(uid));
        let group = Identity::UnixGroup(Gid(10));
        let other = Identity::Other("unix-netgroup".to_owned());
        let cases = [
            (vec![user(0), user(1000), user(1001)], Some(Uid(1000))),
            (vec![user(1001), user(0)], Some(Uid(0))),
            (vec![group.clone(), user(1001), user(1002)], Some(Uid(1001))),
            (vec![group, other], None),
            (vec![], None),
        ];
        for (identities, want) in cases {
            assert_eq!(
                pick_identity(&identities, Uid(1000)),
                want,
                "{identities:?}"
            );
        }
    }

    #[test]
    fn names_from_passwd() {
        let passwd = "root:x:0:0:Super User:/root:/bin/bash\n# ghost:x:7:7::/:/bin/sh\n\
            ada:x:1000:1000:Ada,,,:/home/ada:/bin/zsh\nbroken line\n:x:5:5::/:/bin/sh\n";
        let name = |n: &str| Some(UserName(n.to_owned()));
        let cases = [
            (0, name("root")),
            (1000, name("ada")),
            (7, None),
            (5, None),
            (9, None),
        ];
        for (uid, want) in cases {
            assert_eq!(name_in_passwd(passwd, Uid(uid)), want, "{uid}");
        }
    }
}
