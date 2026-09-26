//! Which of polkitd's identities to ask for (F525): the session's own user when listed, else
//! root, else the first `unix-user`; groups and other kinds are never chosen (polkitd lists a
//! group's members as users when it wants them).

use super::types::{Identity, Uid};

/// The uid to authenticate as, `own` being this session's user.
pub fn pick_identity(identities: &[Identity], own: Uid) -> Option<Uid> {
    todo!("M11 fill: b-polkit ({identities:?} {own:?})")
}

#[cfg(test)]
mod tests {
    use super::pick_identity;
    use crate::polkit_agent::types::{Gid, Identity, Uid};

    #[test]
    #[ignore = "M11 fill: b-polkit"]
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
}
