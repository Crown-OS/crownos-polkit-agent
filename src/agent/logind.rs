use zbus::{proxy, zvariant::OwnedObjectPath};

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait LoginManager {
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<OwnedObjectPath>;

    fn get_user(&self, uid: u32) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
pub trait LoginSession {
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.freedesktop.login1.User",
    default_service = "org.freedesktop.login1"
)]
pub trait LoginUser {
    #[zbus(property)]
    fn display(&self) -> zbus::Result<(String, OwnedObjectPath)>;
}
