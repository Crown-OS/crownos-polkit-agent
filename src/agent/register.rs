use std::{collections::HashMap, env};

use zbus::{Connection, proxy, zvariant::Value};

/// Where the agent object lives on the system bus.
pub const AGENT_PATH: &str = "/org/crownos/PolicyKit1/AuthenticationAgent";

const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

#[proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    fn register_authentication_agent(
        &self,
        subject: &(&str, HashMap<&str, Value<'_>>),
        locale: &str,
        object_path: &str,
    ) -> zbus::Result<()>;
}

/// Makes the object at [`AGENT_PATH`] the agent for logind session `session_id`.
pub async fn register(system: &Connection, session_id: &str) -> zbus::Result<()> {
    let subject = (
        "unix-session",
        HashMap::from([("session-id", Value::from(session_id))]),
    );
    AuthorityProxy::new(system)
        .await?
        .register_authentication_agent(&subject, &locale(), AGENT_PATH)
        .await
}

fn locale() -> String {
    LOCALE_VARIABLES
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.is_empty()))
        .unwrap_or_else(|| "C".to_owned())
}
