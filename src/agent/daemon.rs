use anyhow::{Context, Result};
use tokio::signal::unix::{SignalKind, signal};
use zbus::connection;

use super::{AGENT_PATH, AuthenticationAgent, DialogCommand, register, session};

const DIALOG_PROGRAM: &str = "crownos-polkit-dialog";

/// Serves the agent on the system bus for this session until SIGTERM or SIGINT.
pub async fn run() -> Result<()> {
    let dialog =
        DialogCommand::beside_current_exe(DIALOG_PROGRAM).context("locating the dialog program")?;
    let system = connection::Builder::system()?
        .serve_at(AGENT_PATH, AuthenticationAgent::new(dialog))?
        .build()
        .await
        .context("connecting to the system bus")?;
    let session_id = session::current_session_id(&system)
        .await
        .context("finding the logind session")?;
    register::register(&system, &session_id)
        .await
        .with_context(|| format!("registering as the polkit agent of session {session_id}"))?;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
    Ok(())
}
