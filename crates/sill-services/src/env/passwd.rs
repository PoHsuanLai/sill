//! The seam between the lock screen and the system's accounts database: [`Passwd`], implemented by
//! [`SystemPasswd`] (the real one, `getpwuid_r` through `nix`, so every source the system's name
//! service knows is asked) and [`FakePasswd`] (every test, which never reads the machine's
//! accounts).

use std::fmt;

use nix::unistd::{Uid, User};
use sill_model::auth::{RealName, UserName};

/// A numeric user id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserId(pub u32);

/// The person a session belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: UserId,
    /// The login name.
    pub user: UserName,
    /// The name they go by.
    pub name: RealName,
}

/// Says who this process runs as. Blocking: a name service may be a network directory, so the
/// service calls it on tokio's blocking pool.
pub trait Passwd: fmt::Debug + Send + Sync {
    fn current(&self) -> Person;
}

/// What the accounts database holds for one user.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    login: String,
    /// The comment field, whose first comma-separated part is the person's name.
    gecos: String,
}

/// The person with id `id`: their `entry` when the database has one, else `fallback` for both
/// names (the user the session was started with; empty when that is not known).
fn person_of(id: UserId, entry: Option<Entry>, fallback: &str) -> Person {
    match entry.filter(|entry| !entry.login.is_empty()) {
        Some(Entry { login, gecos }) => {
            let first = gecos.split(',').next().unwrap_or_default().trim();
            let name = if first.is_empty() { &login } else { first };
            Person {
                id,
                name: RealName(name.to_owned()),
                user: UserName(login),
            }
        }
        None => Person {
            id,
            user: UserName(fallback.to_owned()),
            name: RealName(fallback.to_owned()),
        },
    }
}

/// The system's accounts database, for the user this process runs as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPasswd {
    /// The login name the session was started with, used when the database has no entry or
    /// cannot be read; read once at the edge.
    pub fallback_login: String,
}

impl Passwd for SystemPasswd {
    fn current(&self) -> Person {
        let uid = Uid::effective();
        let entry = User::from_uid(uid).ok().flatten().map(|user| Entry {
            login: user.name,
            gecos: user.gecos.to_string_lossy().into_owned(),
        });
        person_of(UserId(uid.as_raw()), entry, &self.fallback_login)
    }
}

/// A stand-in for the accounts database: always this person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakePasswd(pub Person);

impl Passwd for FakePasswd {
    fn current(&self) -> Person {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, FakePasswd, Passwd, Person, UserId, person_of};
    use sill_model::auth::{RealName, UserName};

    fn person(id: u32, user: &str, name: &str) -> Person {
        Person {
            id: UserId(id),
            user: UserName(user.to_owned()),
            name: RealName(name.to_owned()),
        }
    }

    fn entry(login: &str, gecos: &str) -> Option<Entry> {
        Some(Entry {
            login: login.to_owned(),
            gecos: gecos.to_owned(),
        })
    }

    #[test]
    fn an_entry_gives_the_login_and_the_first_part_of_the_comment() {
        let cases = [
            (
                "a name and room details",
                entry("ada", "Ada Lovelace,,,"),
                person(1000, "ada", "Ada Lovelace"),
            ),
            (
                "padded",
                entry("ada", "  Ada Lovelace  ,room"),
                person(1000, "ada", "Ada Lovelace"),
            ),
            (
                "an empty comment",
                entry("bob", ""),
                person(1000, "bob", "bob"),
            ),
            (
                "only commas",
                entry("bob", ",,,"),
                person(1000, "bob", "bob"),
            ),
            ("no entry", None, person(1000, "fallback", "fallback")),
            (
                "an entry without a login",
                entry("", "Nobody"),
                person(1000, "fallback", "fallback"),
            ),
        ];
        for (case, found, want) in cases {
            assert_eq!(person_of(UserId(1000), found, "fallback"), want, "{case}");
        }
    }

    #[test]
    fn no_entry_and_no_fallback_leaves_the_names_empty() {
        assert_eq!(person_of(UserId(7), None, ""), person(7, "", ""));
    }

    #[test]
    fn the_fake_is_always_its_person() {
        let ada = person(1000, "ada", "Ada Lovelace");
        assert_eq!(FakePasswd(ada.clone()).current(), ada);
    }
}
