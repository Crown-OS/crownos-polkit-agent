use zbus::DBusError;

/// The errors polkitd understands from an agent; `Cancelled` tells it the user dismissed the
/// dialog rather than that the agent broke.
#[derive(Debug, DBusError)]
#[zbus(prefix = "org.freedesktop.PolicyKit1.Error")]
pub enum PolkitError {
    #[zbus(error)]
    ZBus(zbus::Error),
    Failed(String),
    Cancelled(String),
}
