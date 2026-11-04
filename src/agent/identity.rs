use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

use super::user_directory::UserDirectory;

/// One polkit identity as it arrives over D-Bus: `(kind, {"uid"|"gid": u32})`.
pub type DbusIdentity = (String, HashMap<String, OwnedValue>);

/// An identity polkit accepts as proof; others (netgroups) cannot answer a password prompt here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    User(u32),
    Group(u32),
}

impl Identity {
    pub fn from_dbus((kind, details): &DbusIdentity) -> Option<Self> {
        let id = |key| details.get(key).and_then(|value| u32::try_from(value).ok());
        match kind.as_str() {
            "unix-user" => id("uid").map(Self::User),
            "unix-group" => id("gid").map(Self::Group),
            _ => None,
        }
    }
}

/// The user names that may authenticate, groups expanded to their members, without repeats:
/// the current user first when allowed, else polkit's order (its first admin).
pub fn candidate_users(identities: &[DbusIdentity], directory: &impl UserDirectory) -> Vec<String> {
    let current = directory.current_user();
    let mut users: Vec<String> = Vec::with_capacity(identities.len());
    let mut add = |name: String| {
        if !users.contains(&name) {
            users.push(name);
        }
    };
    for identity in identities.iter().filter_map(Identity::from_dbus) {
        match identity {
            Identity::User(uid) => directory.user_name(uid).into_iter().for_each(&mut add),
            Identity::Group(gid) => {
                directory.group_members(gid).into_iter().for_each(&mut add);
                if let Some(account) = current
                    .as_ref()
                    .filter(|account| account.primary_gid == gid)
                {
                    add(account.name.clone());
                }
            }
        }
    }
    if let Some(position) =
        current.and_then(|account| users.iter().position(|user| *user == account.name))
    {
        users[..=position].rotate_right(1);
    }
    users
}

#[cfg(test)]
mod tests {
    use zbus::zvariant::Value;

    use super::*;
    use crate::agent::user_directory::Account;

    struct FakeDirectory;

    impl UserDirectory for FakeDirectory {
        fn user_name(&self, uid: u32) -> Option<String> {
            match uid {
                0 => Some("root".into()),
                1000 => Some("alice".into()),
                1001 => Some("bob".into()),
                _ => None,
            }
        }

        fn group_members(&self, gid: u32) -> Vec<String> {
            match gid {
                998 => vec!["bob".into(), "carol".into()],
                _ => Vec::new(),
            }
        }

        fn current_user(&self) -> Option<Account> {
            Some(Account {
                name: "alice".into(),
                primary_gid: 1000,
            })
        }
    }

    fn identity(kind: &str, key: &str, id: u32) -> DbusIdentity {
        let value = OwnedValue::try_from(Value::U32(id)).unwrap();
        (kind.into(), HashMap::from([(key.into(), value)]))
    }

    #[test]
    fn prefers_the_current_user() {
        let identities = [
            identity("unix-user", "uid", 0),
            identity("unix-user", "uid", 1001),
            identity("unix-user", "uid", 1000),
        ];
        assert_eq!(
            candidate_users(&identities, &FakeDirectory),
            ["alice", "root", "bob"]
        );
    }

    #[test]
    fn expands_groups_without_repeats() {
        let identities = [
            identity("unix-user", "uid", 1001),
            identity("unix-group", "gid", 998),
            identity("unix-group", "gid", 1000),
        ];
        assert_eq!(
            candidate_users(&identities, &FakeDirectory),
            ["alice", "bob", "carol"]
        );
    }

    #[test]
    fn skips_unknown_and_malformed_identities() {
        let identities = [
            identity("unix-netgroup", "name", 1),
            identity("unix-user", "gid", 1000),
            identity("unix-user", "uid", 4242),
            identity("unix-user", "uid", 0),
        ];
        assert_eq!(candidate_users(&identities, &FakeDirectory), ["root"]);
    }
}
