#![allow(clippy::unwrap_used)]

use std::{collections::HashMap, time::Duration};

use crownos_polkit_agent::agent::{AGENT_PATH, AuthenticationAgent, DialogCommand};
use nix::unistd::getuid;
use tokio::{net::UnixStream, time::timeout};
use zbus::{
    Connection, Guid,
    connection::Builder,
    zvariant::{OwnedValue, Value},
};

const INTERFACE: &str = "org.freedesktop.PolicyKit1.AuthenticationAgent";
const CANCELLED: &str = "org.freedesktop.PolicyKit1.Error.Cancelled";
const DEADLINE: Duration = Duration::from_secs(5);

async fn connect(dialog: DialogCommand) -> (Connection, Connection) {
    let (server_stream, client_stream) = UnixStream::pair().unwrap();
    let server = Builder::unix_stream(server_stream)
        .server(Guid::generate())
        .unwrap()
        .p2p()
        .serve_at(AGENT_PATH, AuthenticationAgent::new(dialog))
        .unwrap()
        .build();
    let client = Builder::unix_stream(client_stream).p2p().build();
    tokio::try_join!(server, client).unwrap()
}

async fn begin(client: &Connection, cookie: &str) -> zbus::Result<()> {
    let uid = OwnedValue::try_from(Value::U32(getuid().as_raw())).unwrap();
    let identities = vec![("unix-user", HashMap::from([("uid", uid)]))];
    let details: HashMap<&str, &str> = HashMap::new();
    client
        .call_method(
            None::<&str>,
            AGENT_PATH,
            Some(INTERFACE),
            "BeginAuthentication",
            &(
                "org.crownos.test",
                "Authenticate to test",
                "drive-harddisk",
                details,
                cookie,
                identities,
            ),
        )
        .await
        .map(drop)
}

async fn cancel(client: &Connection, cookie: &str) {
    client
        .call_method(
            None::<&str>,
            AGENT_PATH,
            Some(INTERFACE),
            "CancelAuthentication",
            &(cookie,),
        )
        .await
        .unwrap();
}

fn assert_cancelled(result: zbus::Result<()>) {
    match result {
        Err(zbus::Error::MethodError(name, _, _)) => assert_eq!(name.as_str(), CANCELLED),
        other => panic!("expected {CANCELLED}, got {other:?}"),
    }
}

#[tokio::test]
async fn replies_once_the_dialog_succeeds() {
    let (_server, client) = connect(DialogCommand::new("true")).await;
    timeout(DEADLINE, begin(&client, "ok"))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_dismissed_dialog_is_cancelled() {
    let (_server, client) = connect(DialogCommand::new("false")).await;
    assert_cancelled(timeout(DEADLINE, begin(&client, "no")).await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_kills_the_dialog_and_frees_the_queue() {
    let (_server, client) = connect(DialogCommand::new("sleep").arg("30")).await;
    let showing = tokio::spawn({
        let client = client.clone();
        async move { begin(&client, "showing").await }
    });
    let waiting = tokio::spawn({
        let client = client.clone();
        async move { begin(&client, "waiting").await }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    cancel(&client, "waiting").await;
    assert_cancelled(timeout(DEADLINE, waiting).await.unwrap().unwrap());
    cancel(&client, "showing").await;
    assert_cancelled(timeout(DEADLINE, showing).await.unwrap().unwrap());
}
