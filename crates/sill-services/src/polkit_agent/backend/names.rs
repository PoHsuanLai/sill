//! Naming the users polkit offers: the seam to the system's user database ([`Names`]), the real
//! one ([`SystemNames`], `getpwuid_r` through `nix`, so accounts that come from a directory
//! service (LDAP, sssd, systemd-homed) are named like local ones) and the choice of which name to
//! show ([`user_for`], pure over the seam).

use std::fmt;

use nix::unistd::User;
use sill_model::auth::UserName;
use sill_model::polkit_agent::{Identity, Uid, pick_identity};

/// The login name of a user id. Blocking: a name service may be a network directory, so the
/// agent's loop calls it on tokio's blocking pool.
pub(crate) trait Names: fmt::Debug + Send + Sync {
    /// `None` when the database has no such user, or the user has an empty name.
    fn name_of(&self, uid: Uid) -> Option<UserName>;
}

/// The system's user database, through the system's name service.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemNames;

impl Names for SystemNames {
    fn name_of(&self, uid: Uid) -> Option<UserName> {
        User::from_uid(nix::unistd::Uid::from_raw(uid.0))
            .ok()
            .flatten()
            .map(|user| user.name)
            .filter(|name| !name.is_empty())
            .map(UserName)
    }
}

/// This session's user.
#[derive(Debug, Clone)]
pub(crate) struct Own {
    pub(crate) uid: Uid,
    pub(crate) name: UserName,
}

/// The identity to ask for, named; `None` when polkitd offers no user, or the chosen one has no
/// name in the user database.
pub(crate) fn user_for(identities: &[Identity], own: &Own, names: &impl Names) -> Option<UserName> {
    let uid = pick_identity(identities, own.uid)?;
    match uid == own.uid {
        true => Some(own.name.clone()),
        false => names.name_of(uid),
    }
}

/// This process's user: its uid (the owner of `/proc/self`), named from the user database, else
/// `$USER`.
pub(crate) fn own(names: &impl Names) -> Own {
    use std::os::unix::fs::MetadataExt;
    let uid = Uid(std::fs::metadata("/proc/self").map_or(u32::MAX, |m| m.uid()));
    let name = names
        .name_of(uid)
        .or_else(|| std::env::var("USER").ok().map(UserName))
        .unwrap_or_default();
    Own { uid, name }
}

#[cfg(test)]
mod tests {
    use super::{Names, Own, user_for};
    use sill_model::auth::UserName;
    use sill_model::polkit_agent::{Gid, Identity, Uid};

    /// A user database of a few fixed users, never the machine's.
    #[derive(Debug)]
    struct Fixed;

    impl Names for Fixed {
        fn name_of(&self, uid: Uid) -> Option<UserName> {
            match uid.0 {
                0 => Some(UserName("root".to_owned())),
                1001 => Some(UserName("grace".to_owned())),
                _ => None,
            }
        }
    }

    #[test]
    fn the_chosen_identity_is_named_by_the_database_unless_it_is_ours() {
        let own = Own {
            uid: Uid(1000),
            name: UserName("ada".to_owned()),
        };
        let user = |uid| Identity::UnixUser(Uid(uid));
        let name = |n: &str| Some(UserName(n.to_owned()));
        let cases = [
            (
                "ours is named without a lookup",
                vec![user(1000), user(0)],
                name("ada"),
            ),
            ("root next", vec![user(1001), user(0)], name("root")),
            (
                "the first user otherwise",
                vec![user(1001), user(1002)],
                name("grace"),
            ),
            ("a user the database lacks", vec![user(1002)], None),
            (
                "groups are never chosen",
                vec![Identity::UnixGroup(Gid(10))],
                None,
            ),
            ("nobody offered", vec![], None),
        ];
        for (what, identities, want) in cases {
            assert_eq!(user_for(&identities, &own, &Fixed), want, "{what}");
        }
    }
}
