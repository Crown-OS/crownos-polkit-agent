use nix::unistd::{Gid, Group, Uid, User, getuid};

/// A local account: its login name and primary group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    pub primary_gid: u32,
}

/// Where user and group names come from: NSS on a real system, a table in tests.
pub trait UserDirectory {
    fn user_name(&self, uid: u32) -> Option<String>;
    fn group_members(&self, gid: u32) -> Vec<String>;
    fn current_user(&self) -> Option<Account>;
}

/// The system's users through `getpwuid_r` / `getgrgid_r`, so LDAP and homed users count too.
pub struct SystemUsers;

impl UserDirectory for SystemUsers {
    fn user_name(&self, uid: u32) -> Option<String> {
        User::from_uid(Uid::from_raw(uid))
            .ok()
            .flatten()
            .map(|user| user.name)
    }

    fn group_members(&self, gid: u32) -> Vec<String> {
        Group::from_gid(Gid::from_raw(gid))
            .ok()
            .flatten()
            .map(|group| group.mem)
            .unwrap_or_default()
    }

    fn current_user(&self) -> Option<Account> {
        User::from_uid(getuid()).ok().flatten().map(|user| Account {
            name: user.name,
            primary_gid: user.gid.as_raw(),
        })
    }
}
