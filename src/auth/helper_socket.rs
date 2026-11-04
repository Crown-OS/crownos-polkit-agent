use tokio::net::UnixStream;

/// polkit ≥ 126 starts a helper instance per connection here, so no setuid binary is needed.
const SOCKET_PATH: &str = "/run/polkit/agent-helper.socket";

/// The socket-activated helper, or `None` when it is not installed or not listening.
pub async fn connect() -> Option<UnixStream> {
    UnixStream::connect(SOCKET_PATH).await.ok()
}
